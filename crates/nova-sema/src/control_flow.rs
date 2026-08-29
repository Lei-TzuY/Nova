//! Verified function-level control-flow graphs for semantic dataflow.
//!
//! The bootstrap analyzer builds these graphs while lowering source so recovery-only
//! paths remain visible without being allowed to export facts into reachable code.

use crate::hir::{Binding, BindingId, FunctionId};
use nova_diagnostics::Diagnostic;
use nova_source::Span;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Control-flow graphs for one analyzed source file, in function order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlFlowProgram {
    functions: Vec<FunctionControlFlow>,
}

impl ControlFlowProgram {
    pub(crate) fn new(functions: Vec<FunctionControlFlow>) -> Self {
        Self { functions }
    }

    /// Returns verified graphs in HIR function order.
    #[must_use]
    pub fn functions(&self) -> &[FunctionControlFlow] {
        &self.functions
    }
}

/// Stable graph-local identity for one control-flow node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FlowNodeId(usize);

impl FlowNodeId {
    /// Returns the graph-local node index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// One verified function-level control-flow graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionControlFlow {
    function: FunctionId,
    entry: FlowNodeId,
    nodes: Vec<FlowNode>,
    bindings: Vec<FlowBinding>,
    normal_exits: Vec<FlowNodeId>,
}

impl FunctionControlFlow {
    /// Returns the HIR function represented by this graph.
    #[must_use]
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    /// Returns the unique graph entry.
    #[must_use]
    pub const fn entry(&self) -> FlowNodeId {
        self.entry
    }

    /// Returns nodes in deterministic semantic-lowering order.
    #[must_use]
    pub fn nodes(&self) -> &[FlowNode] {
        &self.nodes
    }

    /// Returns function bindings in semantic identity order.
    #[must_use]
    pub fn bindings(&self) -> &[FlowBinding] {
        &self.bindings
    }

    /// Returns exits that complete the function body normally.
    #[must_use]
    pub fn normal_exits(&self) -> &[FlowNodeId] {
        &self.normal_exits
    }
}

/// Declaration metadata needed by control-flow diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowBinding {
    /// Semantic binding identity.
    pub id: BindingId,
    /// Declared spelling.
    pub name: String,
    /// Exact declaration-name span.
    pub span: Span,
}

/// One node in a function control-flow graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowNode {
    /// Graph-local node identity.
    pub id: FlowNodeId,
    /// Semantic action performed at this node.
    pub kind: FlowNodeKind,
    /// Incoming graph edges.
    pub predecessors: Vec<FlowEdge>,
    /// Source range associated with the action, when present.
    pub span: Option<Span>,
}

/// One predecessor edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowEdge {
    /// Predecessor node.
    pub from: FlowNodeId,
    /// Why this edge exists.
    pub kind: FlowEdgeKind,
}

/// Control-flow edge categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowEdgeKind {
    /// A path that may contribute facts to reachable continuation.
    Execution,
    /// Source checked for diagnostics whose facts are discarded afterward.
    Diagnostic,
    /// A loop fallthrough or `continue` edge back to the loop header.
    Backedge,
}

/// Semantic actions represented in the bootstrap CFG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlowNodeKind {
    /// Unique function entry.
    Entry,
    /// Explicit branch path, including match arms and loop bodies.
    Branch,
    /// Intersection point for continuing predecessor paths.
    Join,
    /// A binding becomes definitely initialized after this node.
    Initialize(BindingId),
    /// A resolved binding is read at this node.
    Read(BindingId),
    /// A non-continuing control transfer.
    Transfer(FlowTransfer),
    /// Normal completion of the function body.
    Exit,
}

/// Non-continuing transfers represented in the graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowTransfer {
    /// Explicit function return.
    Return,
    /// Exit from the nearest loop.
    Break,
    /// Start the nearest loop's next condition test.
    Continue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FlowError {
    message: String,
    span: Span,
}

impl FlowError {
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    pub(crate) const fn span(&self) -> Span {
        self.span
    }

    fn invalid(span: Span, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

pub(crate) struct FunctionFlowBuilder {
    function: FunctionId,
    span: Span,
    entry: FlowNodeId,
    cursor: FlowNodeId,
    nodes: Vec<FlowNode>,
    bindings: BTreeMap<BindingId, FlowBinding>,
    build_error: Option<FlowError>,
}

impl FunctionFlowBuilder {
    pub(crate) fn new(function: FunctionId, span: Span) -> Self {
        let entry = FlowNodeId(0);
        Self {
            function,
            span,
            entry,
            cursor: entry,
            nodes: vec![FlowNode {
                id: entry,
                kind: FlowNodeKind::Entry,
                predecessors: Vec::new(),
                span: Some(span),
            }],
            bindings: BTreeMap::new(),
            build_error: None,
        }
    }

    pub(crate) const fn cursor(&self) -> FlowNodeId {
        self.cursor
    }

    pub(crate) fn set_cursor(&mut self, cursor: FlowNodeId) {
        self.cursor = cursor;
    }

    pub(crate) fn register_binding(&mut self, binding: &Binding) {
        self.bindings
            .entry(binding.id)
            .or_insert_with(|| FlowBinding {
                id: binding.id,
                name: binding.name.clone(),
                span: binding.span,
            });
    }

    pub(crate) fn advance(
        &mut self,
        kind: FlowNodeKind,
        span: Option<Span>,
        edge_kind: FlowEdgeKind,
    ) -> FlowNodeId {
        let predecessor = self.cursor;
        let node = self.push_node(
            kind,
            span,
            vec![FlowEdge {
                from: predecessor,
                kind: edge_kind,
            }],
        );
        self.cursor = node;
        node
    }

    pub(crate) fn fork_from(
        &mut self,
        predecessor: FlowNodeId,
        span: Option<Span>,
        edge_kind: FlowEdgeKind,
    ) -> FlowNodeId {
        let node = self.push_node(
            FlowNodeKind::Branch,
            span,
            vec![FlowEdge {
                from: predecessor,
                kind: edge_kind,
            }],
        );
        self.cursor = node;
        node
    }

    pub(crate) fn join(
        &mut self,
        predecessors: impl IntoIterator<Item = FlowNodeId>,
        span: Option<Span>,
        edge_kind: FlowEdgeKind,
    ) -> FlowNodeId {
        let mut seen = BTreeSet::new();
        let predecessors = predecessors
            .into_iter()
            .filter(|predecessor| seen.insert(*predecessor))
            .map(|from| FlowEdge {
                from,
                kind: edge_kind,
            })
            .collect();
        let node = self.push_node(FlowNodeKind::Join, span, predecessors);
        self.cursor = node;
        node
    }

    pub(crate) fn add_backedge(&mut self, from: FlowNodeId, to: FlowNodeId) {
        if from.index() >= self.nodes.len() || to.index() >= self.nodes.len() {
            self.build_error.get_or_insert_with(|| {
                FlowError::invalid(self.span, "loop backedge endpoint is out of range")
            });
            return;
        }
        let node = &mut self.nodes[to.index()];
        let edge = FlowEdge {
            from,
            kind: FlowEdgeKind::Backedge,
        };
        if !node.predecessors.contains(&edge) {
            node.predecessors.push(edge);
        }
    }

    pub(crate) fn finish(
        mut self,
        normal_exit: Option<FlowNodeId>,
    ) -> Result<FunctionControlFlow, FlowError> {
        if let Some(error) = self.build_error.take() {
            return Err(error);
        }
        let normal_exits = if let Some(exit) = normal_exit {
            self.set_cursor(exit);
            let exit = self.advance(FlowNodeKind::Exit, Some(self.span), FlowEdgeKind::Execution);
            vec![exit]
        } else {
            Vec::new()
        };
        let graph = FunctionControlFlow {
            function: self.function,
            entry: self.entry,
            nodes: self.nodes,
            bindings: self.bindings.into_values().collect(),
            normal_exits,
        };
        verify(&graph, self.span)?;
        Ok(graph)
    }

    fn push_node(
        &mut self,
        kind: FlowNodeKind,
        span: Option<Span>,
        predecessors: Vec<FlowEdge>,
    ) -> FlowNodeId {
        let id = FlowNodeId(self.nodes.len());
        self.nodes.push(FlowNode {
            id,
            kind,
            predecessors,
            span,
        });
        id
    }
}

pub(crate) fn definite_initialization_diagnostics(
    graph: &FunctionControlFlow,
    fallback_span: Span,
) -> Result<Vec<Diagnostic>, FlowError> {
    verify(graph, fallback_span)?;
    let bindings = graph
        .bindings
        .iter()
        .map(|binding| (binding.id, binding))
        .collect::<BTreeMap<_, _>>();
    let universe = bindings.keys().copied().collect::<BTreeSet<_>>();
    let mut outputs = vec![universe; graph.nodes.len()];
    let mut inputs = outputs.clone();
    outputs[graph.entry.index()] = BTreeSet::new();
    inputs[graph.entry.index()] = BTreeSet::new();

    loop {
        let mut changed = false;
        for node in &graph.nodes {
            let incoming = if node.id == graph.entry {
                BTreeSet::new()
            } else {
                intersect_predecessors(node, &outputs, fallback_span)?
            };
            let mut outgoing = incoming.clone();
            if let FlowNodeKind::Initialize(binding) = &node.kind {
                outgoing.insert(*binding);
            }
            let index = node.id.index();
            if inputs[index] != incoming || outputs[index] != outgoing {
                inputs[index] = incoming;
                outputs[index] = outgoing;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut diagnostics = Vec::new();
    for node in &graph.nodes {
        let FlowNodeKind::Read(binding_id) = &node.kind else {
            continue;
        };
        let binding = bindings.get(binding_id).ok_or_else(|| {
            FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!("read references unknown binding:{}", binding_id.index()),
            )
        })?;
        if !inputs[node.id.index()].contains(binding_id) {
            diagnostics.push(
                Diagnostic::error("N3009", "binding may be uninitialized")
                    .with_primary(
                        node.span.unwrap_or(fallback_span),
                        format!(
                            "`{}` is not definitely initialized on this path",
                            binding.name
                        ),
                    )
                    .with_secondary(binding.span, "binding declared here"),
            );
        }
    }
    Ok(diagnostics)
}

fn intersect_predecessors(
    node: &FlowNode,
    outputs: &[BTreeSet<BindingId>],
    fallback_span: Span,
) -> Result<BTreeSet<BindingId>, FlowError> {
    let mut predecessors = node.predecessors.iter();
    let first = predecessors.next().ok_or_else(|| {
        FlowError::invalid(
            node.span.unwrap_or(fallback_span),
            format!("flow node {} has no predecessor", node.id.index()),
        )
    })?;
    let mut result = outputs
        .get(first.from.index())
        .cloned()
        .ok_or_else(|| FlowError::invalid(fallback_span, "flow predecessor is out of range"))?;
    for predecessor in predecessors {
        let facts = outputs
            .get(predecessor.from.index())
            .ok_or_else(|| FlowError::invalid(fallback_span, "flow predecessor is out of range"))?;
        result.retain(|binding| facts.contains(binding));
    }
    Ok(result)
}

fn verify(graph: &FunctionControlFlow, fallback_span: Span) -> Result<(), FlowError> {
    if graph.entry.index() >= graph.nodes.len() {
        return Err(FlowError::invalid(
            fallback_span,
            "flow entry is out of range",
        ));
    }
    let mut successors = vec![Vec::<(FlowNodeId, FlowEdgeKind)>::new(); graph.nodes.len()];
    for (index, node) in graph.nodes.iter().enumerate() {
        if node.id.index() != index {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!("flow node identity at slot {index} is {}", node.id.index()),
            ));
        }
        if node.id == graph.entry {
            if !node.predecessors.is_empty() {
                return Err(FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    "flow entry has a predecessor",
                ));
            }
        } else if node.predecessors.is_empty() {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!("flow node {index} has no predecessor"),
            ));
        }
        for edge in &node.predecessors {
            let Some(outgoing) = successors.get_mut(edge.from.index()) else {
                return Err(FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    format!("flow node {index} has an out-of-range predecessor"),
                ));
            };
            outgoing.push((node.id, edge.kind));
        }
    }

    let mut execution_reached = BTreeSet::new();
    let mut execution_queue = VecDeque::from([graph.entry]);
    while let Some(node) = execution_queue.pop_front() {
        if !execution_reached.insert(node) {
            continue;
        }
        if let Some(next) = successors.get(node.index()) {
            execution_queue.extend(
                next.iter()
                    .filter(|(_, edge)| *edge != FlowEdgeKind::Diagnostic)
                    .map(|(successor, _)| *successor),
            );
        }
    }
    for node in &graph.nodes {
        if !execution_reached.contains(&node.id) {
            continue;
        }
        if let Some(edge) = node.predecessors.iter().find(|edge| {
            edge.kind == FlowEdgeKind::Diagnostic || !execution_reached.contains(&edge.from)
        }) {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!(
                    "diagnostic-only control flow from node {} reconnects to executable node {}",
                    edge.from.index(),
                    node.id.index()
                ),
            ));
        }
    }

    let known_bindings = graph
        .bindings
        .iter()
        .map(|binding| binding.id)
        .collect::<BTreeSet<_>>();
    for node in &graph.nodes {
        let binding = match &node.kind {
            FlowNodeKind::Initialize(binding) | FlowNodeKind::Read(binding) => Some(*binding),
            _ => None,
        };
        if binding.is_some_and(|binding| !known_bindings.contains(&binding)) {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!(
                    "flow node {} references an unknown binding",
                    node.id.index()
                ),
            ));
        }

        let invalid_successor =
            successors[node.id.index()]
                .iter()
                .any(|(_, edge)| match &node.kind {
                    FlowNodeKind::Transfer(FlowTransfer::Return) => {
                        *edge != FlowEdgeKind::Diagnostic
                    }
                    FlowNodeKind::Exit => true,
                    FlowNodeKind::Transfer(FlowTransfer::Continue) => {
                        !matches!(edge, FlowEdgeKind::Diagnostic | FlowEdgeKind::Backedge)
                    }
                    FlowNodeKind::Transfer(FlowTransfer::Break) => *edge == FlowEdgeKind::Backedge,
                    _ => false,
                });
        if invalid_successor {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!(
                    "flow node {} has a successor incompatible with its transfer",
                    node.id.index()
                ),
            ));
        }
    }

    let mut reached = BTreeSet::new();
    let mut queue = VecDeque::from([graph.entry]);
    while let Some(node) = queue.pop_front() {
        if !reached.insert(node) {
            continue;
        }
        if let Some(next) = successors.get(node.index()) {
            queue.extend(next.iter().map(|(successor, _)| *successor));
        }
    }
    if reached.len() != graph.nodes.len() {
        return Err(FlowError::invalid(
            fallback_span,
            "control-flow graph contains a node unreachable from its entry",
        ));
    }
    let actual_exits = graph
        .nodes
        .iter()
        .filter_map(|node| matches!(node.kind, FlowNodeKind::Exit).then_some(node.id))
        .collect::<BTreeSet<_>>();
    let declared_exits = graph.normal_exits.iter().copied().collect::<BTreeSet<_>>();
    if declared_exits.len() != graph.normal_exits.len() {
        return Err(FlowError::invalid(
            fallback_span,
            "normal exit table contains duplicate entries",
        ));
    }
    if declared_exits != actual_exits {
        return Err(FlowError::invalid(
            fallback_span,
            "normal exit table does not exactly match exit nodes",
        ));
    }
    if let Some(exit) = declared_exits
        .iter()
        .find(|exit| !execution_reached.contains(exit))
    {
        return Err(FlowError::invalid(
            graph.nodes[exit.index()].span.unwrap_or(fallback_span),
            format!(
                "normal exit node {} is not executable-reachable",
                exit.index()
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        FlowEdgeKind, FlowNodeKind, FlowTransfer, FunctionFlowBuilder,
        definite_initialization_diagnostics,
    };
    use crate::hir::{Binding, BindingId, FunctionId, Type};
    use nova_source::{SourceId, Span};

    fn span(start: usize, end: usize) -> Span {
        Span::new(SourceId::new(0), start, end).expect("valid test span")
    }

    fn binding(index: usize, name: &str, at: usize) -> Binding {
        Binding {
            id: BindingId::new(index),
            name: name.to_owned(),
            ty: Type::Int,
            mutable: true,
            span: span(at, at + name.len()),
        }
    }

    #[test]
    fn joins_initialization_by_intersection() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let value = binding(0, "value", 1);
        builder.register_binding(&value);
        let entry = builder.cursor();

        builder.fork_from(entry, None, FlowEdgeKind::Execution);
        builder.advance(
            FlowNodeKind::Initialize(value.id),
            Some(value.span),
            FlowEdgeKind::Execution,
        );
        let initialized = builder.cursor();
        let untouched = builder.fork_from(entry, None, FlowEdgeKind::Execution);
        let join = builder.join([initialized, untouched], None, FlowEdgeKind::Execution);
        assert_ne!(join, initialized);
        builder.advance(
            FlowNodeKind::Read(value.id),
            Some(span(10, 15)),
            FlowEdgeKind::Execution,
        );

        let exit = builder.cursor();
        let graph = builder.finish(Some(exit)).expect("valid graph");
        let diagnostics =
            definite_initialization_diagnostics(&graph, span(0, 20)).expect("valid dataflow");
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn backedges_do_not_erase_first_entry_path() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let value = binding(0, "value", 1);
        builder.register_binding(&value);
        let preheader = builder.cursor();
        let header = builder.join([preheader], None, FlowEdgeKind::Execution);
        builder.advance(
            FlowNodeKind::Read(value.id),
            Some(span(5, 10)),
            FlowEdgeKind::Execution,
        );
        builder.advance(
            FlowNodeKind::Initialize(value.id),
            Some(span(12, 17)),
            FlowEdgeKind::Execution,
        );
        builder.add_backedge(builder.cursor(), header);

        let graph = builder.finish(None).expect("valid cyclic graph");
        let diagnostics =
            definite_initialization_diagnostics(&graph, span(0, 20)).expect("valid dataflow");
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn verifier_rejects_execution_after_return() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        builder.advance(
            FlowNodeKind::Transfer(FlowTransfer::Return),
            Some(span(1, 7)),
            FlowEdgeKind::Execution,
        );
        builder.advance(
            FlowNodeKind::Branch,
            Some(span(8, 9)),
            FlowEdgeKind::Execution,
        );

        let error = builder
            .finish(None)
            .expect_err("return cannot have an execution successor");
        assert!(error.message().contains("incompatible"));
    }

    #[test]
    fn verifier_rejects_diagnostic_only_reconnection() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let header = builder.join([entry], None, FlowEdgeKind::Execution);
        let recovery = builder.fork_from(header, Some(span(1, 2)), FlowEdgeKind::Diagnostic);
        builder.add_backedge(recovery, header);

        let error = builder
            .finish(None)
            .expect_err("diagnostic-only recovery must not reconnect to executable flow");
        assert!(error.message().contains("diagnostic-only"));
    }

    #[test]
    fn verifier_rejects_diagnostic_predecessor_on_executable_join() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let left = builder.fork_from(entry, Some(span(1, 2)), FlowEdgeKind::Execution);
        let right = builder.fork_from(entry, Some(span(3, 4)), FlowEdgeKind::Execution);
        let join = builder.join([left, right], Some(span(5, 6)), FlowEdgeKind::Execution);
        let graph_exit = builder.cursor();
        let mut graph = builder.finish(Some(graph_exit)).expect("valid seed graph");
        graph.nodes[join.index()].predecessors[0].kind = FlowEdgeKind::Diagnostic;

        let error = super::verify(&graph, span(0, 20))
            .expect_err("executable continuation cannot consume a diagnostic predecessor");
        assert!(error.message().contains("diagnostic-only"));
    }

    #[test]
    fn verifier_rejects_diagnostic_only_normal_exit() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let recovery = builder.fork_from(entry, Some(span(1, 2)), FlowEdgeKind::Diagnostic);

        let error = builder
            .finish(Some(recovery))
            .expect_err("a normal exit must be reachable without crossing diagnostic flow");
        assert!(error.message().contains("normal exit"));
    }

    #[test]
    fn verifier_rejects_unlisted_exit_node() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        builder.advance(
            FlowNodeKind::Branch,
            Some(span(1, 2)),
            FlowEdgeKind::Execution,
        );
        let graph_exit = builder.cursor();
        let mut graph = builder.finish(Some(graph_exit)).expect("valid seed graph");
        graph.normal_exits.clear();

        let error = super::verify(&graph, span(0, 20))
            .expect_err("every Exit node must appear in the normal-exit table");
        assert!(error.message().contains("normal exit"));
    }

    #[test]
    fn verifier_rejects_successor_after_exit() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        builder.advance(
            FlowNodeKind::Branch,
            Some(span(1, 2)),
            FlowEdgeKind::Execution,
        );
        let graph_exit = builder.cursor();
        let mut graph = builder.finish(Some(graph_exit)).expect("valid seed graph");
        let exit = graph.normal_exits[0];
        let successor = super::FlowNodeId(graph.nodes.len());
        graph.nodes.push(super::FlowNode {
            id: successor,
            kind: FlowNodeKind::Branch,
            predecessors: vec![super::FlowEdge {
                from: exit,
                kind: FlowEdgeKind::Diagnostic,
            }],
            span: Some(span(3, 4)),
        });

        let error = super::verify(&graph, span(0, 20))
            .expect_err("a function Exit must be terminal even for diagnostic source");
        assert!(error.message().contains("successor"));
    }

    #[test]
    fn builder_fails_closed_on_an_invalid_backedge_endpoint() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        builder.add_backedge(builder.cursor(), super::FlowNodeId(99));
        assert!(builder.finish(None).is_err());
    }

    #[test]
    fn verifier_rejects_corrupted_identity_range_reachability_binding_and_exit() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        builder.advance(
            FlowNodeKind::Branch,
            Some(span(1, 2)),
            FlowEdgeKind::Execution,
        );
        let exit = builder.cursor();
        let graph = builder.finish(Some(exit)).expect("valid seed graph");

        let mut wrong_identity = graph.clone();
        wrong_identity.nodes[1].id = super::FlowNodeId(99);
        assert!(super::verify(&wrong_identity, span(0, 20)).is_err());

        let mut out_of_range = graph.clone();
        out_of_range.nodes[1].predecessors[0].from = super::FlowNodeId(99);
        assert!(super::verify(&out_of_range, span(0, 20)).is_err());

        let mut unreachable = graph.clone();
        let isolated = super::FlowNodeId(unreachable.nodes.len());
        unreachable.nodes.push(super::FlowNode {
            id: isolated,
            kind: FlowNodeKind::Branch,
            predecessors: vec![super::FlowEdge {
                from: isolated,
                kind: FlowEdgeKind::Execution,
            }],
            span: Some(span(3, 4)),
        });
        assert!(super::verify(&unreachable, span(0, 20)).is_err());

        let mut unknown_binding = graph.clone();
        unknown_binding.nodes[1].kind = FlowNodeKind::Read(BindingId::new(99));
        assert!(super::verify(&unknown_binding, span(0, 20)).is_err());

        let mut wrong_exit = graph;
        wrong_exit.normal_exits = vec![wrong_exit.entry];
        assert!(super::verify(&wrong_exit, span(0, 20)).is_err());
    }
}
