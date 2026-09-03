//! Verified function-level control-flow graphs for semantic dataflow.
//!
//! The bootstrap analyzer builds these graphs while lowering source so recovery-only
//! paths remain visible without being allowed to export facts into reachable code.

use crate::hir::{Binding, BindingId, ClosureId, FunctionId};
use nova_diagnostics::Diagnostic;
use nova_source::Span;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Control-flow graphs for one analyzed source file, in function order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlFlowProgram {
    functions: Vec<FunctionControlFlow>,
    closures: Vec<ClosureControlFlow>,
}

impl ControlFlowProgram {
    pub(crate) fn new(
        functions: Vec<FunctionControlFlow>,
        closures: Vec<ClosureControlFlow>,
    ) -> Self {
        Self {
            functions,
            closures,
        }
    }

    /// Returns verified graphs in HIR function order.
    #[must_use]
    pub fn functions(&self) -> &[FunctionControlFlow] {
        &self.functions
    }

    /// Returns verified graphs in closure semantic-traversal order.
    #[must_use]
    pub fn closures(&self) -> &[ClosureControlFlow] {
        &self.closures
    }
}

/// One verified closure-level control-flow graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureControlFlow {
    closure: ClosureId,
    graph: FunctionControlFlow,
}

impl ClosureControlFlow {
    /// Returns the HIR closure represented by this graph.
    #[must_use]
    pub const fn closure(&self) -> ClosureId {
        self.closure
    }

    /// Returns the unique graph entry.
    #[must_use]
    pub const fn entry(&self) -> FlowNodeId {
        self.graph.entry()
    }

    /// Returns nodes in deterministic semantic-lowering order.
    #[must_use]
    pub fn nodes(&self) -> &[FlowNode] {
        self.graph.nodes()
    }

    /// Returns closure bindings and captures in semantic identity order.
    #[must_use]
    pub fn bindings(&self) -> &[FlowBinding] {
        self.graph.bindings()
    }

    /// Returns exits that complete the closure body normally.
    #[must_use]
    pub fn normal_exits(&self) -> &[FlowNodeId] {
        self.graph.normal_exits()
    }

    pub(crate) const fn graph(&self) -> &FunctionControlFlow {
        &self.graph
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

    pub(crate) fn new_closure(closure: ClosureId, span: Span) -> Self {
        Self::new(FunctionId::new(closure.index()), span)
    }

    pub(crate) const fn cursor(&self) -> FlowNodeId {
        self.cursor
    }

    pub(crate) fn cursor_is_transfer(&self) -> bool {
        self.nodes
            .get(self.cursor.index())
            .is_some_and(|node| matches!(node.kind, FlowNodeKind::Transfer(_)))
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

    pub(crate) fn finish_closure(
        self,
        closure: ClosureId,
        normal_exit: Option<FlowNodeId>,
    ) -> Result<ClosureControlFlow, FlowError> {
        self.finish(normal_exit)
            .map(|graph| ClosureControlFlow { closure, graph })
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

fn initialization_reads_target(
    graph: &FunctionControlFlow,
    node: &FlowNode,
    binding: BindingId,
    inputs: &[BTreeSet<BindingId>],
) -> bool {
    let Some(target_span) = node.span else {
        return false;
    };
    let mut pending = node
        .predecessors
        .iter()
        .filter(|edge| edge.kind == FlowEdgeKind::Execution)
        .map(|edge| edge.from)
        .collect::<VecDeque<_>>();
    let mut seen = BTreeSet::new();

    while let Some(predecessor_id) = pending.pop_front() {
        if !seen.insert(predecessor_id) {
            continue;
        }
        let predecessor = &graph.nodes[predecessor_id.index()];
        if matches!(predecessor.kind, FlowNodeKind::Read(read) if read == binding)
            && predecessor.span.is_some_and(|read_span| {
                read_span.source() == target_span.source() && read_span.start() >= target_span.end()
            })
            && !inputs[predecessor.id.index()].contains(&binding)
        {
            return true;
        }
        pending.extend(
            predecessor
                .predecessors
                .iter()
                .filter(|edge| edge.kind == FlowEdgeKind::Execution)
                .map(|edge| edge.from),
        );
    }

    false
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
                if !initialization_reads_target(graph, node, *binding, &inputs) {
                    outgoing.insert(*binding);
                }
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

pub(crate) fn unreachable_code_diagnostics(
    graph: &FunctionControlFlow,
    fallback_span: Span,
) -> Result<Vec<Diagnostic>, FlowError> {
    verify(graph, fallback_span)?;

    let mut successors = vec![Vec::<(FlowNodeId, FlowEdgeKind)>::new(); graph.nodes.len()];
    for node in &graph.nodes {
        for edge in &node.predecessors {
            let outgoing = successors.get_mut(edge.from.index()).ok_or_else(|| {
                FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    format!(
                        "flow node {} has an out-of-range predecessor",
                        node.id.index()
                    ),
                )
            })?;
            outgoing.push((node.id, edge.kind));
        }
    }

    let mut execution_reached = BTreeSet::new();
    let mut queue = VecDeque::from([graph.entry]);
    while let Some(node) = queue.pop_front() {
        if !execution_reached.insert(node) {
            continue;
        }
        queue.extend(
            successors[node.index()]
                .iter()
                .filter(|(_, kind)| *kind != FlowEdgeKind::Diagnostic)
                .map(|(successor, _)| *successor),
        );
    }

    let mut warned_spans = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for transfer in &graph.nodes {
        let FlowNodeKind::Transfer(kind) = &transfer.kind else {
            continue;
        };
        if !execution_reached.contains(&transfer.id) {
            continue;
        }

        let first_unreachable = successors[transfer.id.index()]
            .iter()
            .filter(|(_, edge)| *edge == FlowEdgeKind::Diagnostic)
            .filter_map(|(successor, _)| graph.nodes.get(successor.index()))
            .filter_map(|node| node.span)
            .min_by_key(|span| (span.source().raw(), span.start(), span.end()));
        let Some(unreachable_span) = first_unreachable else {
            continue;
        };
        let span_key = (
            unreachable_span.source().raw(),
            unreachable_span.start(),
            unreachable_span.end(),
        );
        if !warned_spans.insert(span_key) {
            continue;
        }

        let reason = match kind {
            FlowTransfer::Return => "this return leaves the function",
            FlowTransfer::Break => "this break leaves the enclosing loop",
            FlowTransfer::Continue => "this continue starts the next loop iteration",
        };
        diagnostics.push(
            Diagnostic::warning("N3033", "unreachable code")
                .with_primary(unreachable_span, "this code cannot be reached")
                .with_secondary(transfer.span.unwrap_or(fallback_span), reason),
        );
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
    if !matches!(graph.nodes[graph.entry.index()].kind, FlowNodeKind::Entry) {
        return Err(FlowError::invalid(
            graph.nodes[graph.entry.index()]
                .span
                .unwrap_or(fallback_span),
            "flow entry does not reference the unique Entry node",
        ));
    }
    if let Some(node) = graph
        .nodes
        .iter()
        .find(|node| matches!(node.kind, FlowNodeKind::Entry) && node.id != graph.entry)
    {
        return Err(FlowError::invalid(
            node.span.unwrap_or(fallback_span),
            "control-flow graph contains more than one Entry node",
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
        for (edge_index, edge) in node.predecessors.iter().enumerate() {
            if node.predecessors[..edge_index].contains(edge) {
                return Err(FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    format!(
                        "flow node {index} contains a duplicate {:?} predecessor from node {}",
                        edge.kind,
                        edge.from.index()
                    ),
                ));
            }
            let Some(outgoing) = successors.get_mut(edge.from.index()) else {
                return Err(FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    format!("flow node {index} has an out-of-range predecessor"),
                ));
            };
            let source = edge.from.index();
            match edge.kind {
                FlowEdgeKind::Backedge if source <= index => {
                    return Err(FlowError::invalid(
                        node.span.unwrap_or(fallback_span),
                        format!(
                            "backedge from node {source} to node {index} is not strictly backward"
                        ),
                    ));
                }
                FlowEdgeKind::Execution | FlowEdgeKind::Diagnostic if source >= index => {
                    return Err(FlowError::invalid(
                        node.span.unwrap_or(fallback_span),
                        format!(
                            "forward {:?} edge from node {source} to node {index} is not strictly forward",
                            edge.kind
                        ),
                    ));
                }
                _ => {}
            }
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

    for node in &graph.nodes {
        let mut has_backedge = false;
        for edge in &node.predecessors {
            if edge.kind != FlowEdgeKind::Backedge {
                continue;
            }
            has_backedge = true;
            if !matches!(node.kind, FlowNodeKind::Join) {
                return Err(FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    format!(
                        "backedge from node {} targets non-Join node {}",
                        edge.from.index(),
                        node.id.index()
                    ),
                ));
            }
            if !execution_reached.contains(&node.id) || !execution_reached.contains(&edge.from) {
                return Err(FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    format!(
                        "backedge from node {} to node {} is not confined to executable control flow",
                        edge.from.index(),
                        node.id.index()
                    ),
                ));
            }
        }
        if has_backedge
            && !node
                .predecessors
                .iter()
                .any(|edge| edge.kind == FlowEdgeKind::Execution && edge.from < node.id)
        {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!(
                    "loop-header Join node {} has no forward Execution predecessor",
                    node.id.index()
                ),
            ));
        }
    }

    for node in &graph.nodes {
        if node.id != graph.entry
            && !matches!(node.kind, FlowNodeKind::Join)
            && node.predecessors.len() != 1
        {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!(
                    "non-Join flow node {} has {} predecessors; expected exactly one",
                    node.id.index(),
                    node.predecessors.len()
                ),
            ));
        }
    }

    for bindings in graph.bindings.windows(2) {
        if bindings[0].id >= bindings[1].id {
            return Err(FlowError::invalid(
                bindings[1].span,
                "flow binding metadata is not in strict semantic identity order",
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
                .any(|(successor, edge)| match &node.kind {
                    FlowNodeKind::Transfer(FlowTransfer::Return) => {
                        *edge != FlowEdgeKind::Diagnostic
                    }
                    FlowNodeKind::Exit => true,
                    FlowNodeKind::Transfer(FlowTransfer::Continue) => {
                        !matches!(edge, FlowEdgeKind::Diagnostic | FlowEdgeKind::Backedge)
                    }
                    FlowNodeKind::Transfer(FlowTransfer::Break) => match edge {
                        FlowEdgeKind::Backedge => true,
                        FlowEdgeKind::Diagnostic => false,
                        FlowEdgeKind::Execution => {
                            !matches!(graph.nodes[successor.index()].kind, FlowNodeKind::Join)
                        }
                    },
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
        definite_initialization_diagnostics, unreachable_code_diagnostics,
    };
    use crate::hir::{Binding, BindingId, FunctionId, Type};
    use nova_diagnostics::Severity;
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
    fn verifier_rejects_loop_header_without_first_entry_predecessor() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 30));
        let value = binding(0, "value", 1);
        builder.register_binding(&value);
        let entry = builder.cursor();
        let header = builder.join([entry], Some(span(2, 3)), FlowEdgeKind::Execution);
        builder.advance(
            FlowNodeKind::Read(value.id),
            Some(span(4, 9)),
            FlowEdgeKind::Execution,
        );
        builder.advance(
            FlowNodeKind::Initialize(value.id),
            Some(span(10, 15)),
            FlowEdgeKind::Execution,
        );
        let loop_path = builder.cursor();

        let alternate = builder.fork_from(entry, Some(span(16, 17)), FlowEdgeKind::Execution);
        builder.advance(
            FlowNodeKind::Initialize(value.id),
            Some(span(18, 23)),
            FlowEdgeKind::Execution,
        );
        let alternate_initialized = builder.cursor();
        let tail = builder.join(
            [loop_path, alternate_initialized],
            Some(span(24, 25)),
            FlowEdgeKind::Execution,
        );
        builder.add_backedge(tail, header);

        let mut graph = builder.finish(None).expect("valid seed cyclic graph");
        let diagnostics = definite_initialization_diagnostics(&graph, span(0, 30))
            .expect("seed graph must verify");
        assert_eq!(diagnostics.len(), 1, "first loop entry must remain visible");

        graph.nodes[header.index()]
            .predecessors
            .retain(|edge| edge.kind == FlowEdgeKind::Backedge);

        let error = super::verify(&graph, span(0, 30))
            .expect_err("a loop header must retain its first-entry execution path");
        assert!(error.message().contains("loop-header"));
        assert!(error.message().contains("Execution predecessor"));
        assert_ne!(alternate, header);
    }

    #[test]
    fn verifier_rejects_break_execution_that_bypasses_a_join() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let transfer = builder.advance(
            FlowNodeKind::Transfer(FlowTransfer::Break),
            Some(span(1, 7)),
            FlowEdgeKind::Execution,
        );
        let join = builder.join([transfer], Some(span(8, 9)), FlowEdgeKind::Execution);
        let mut graph = builder
            .finish(None)
            .expect("valid break-to-join seed graph");

        graph.nodes[join.index()].kind = FlowNodeKind::Branch;

        let error = super::verify(&graph, span(0, 20))
            .expect_err("break execution must re-enter continuation through a Join");
        assert!(error.message().contains("incompatible"));
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
    fn unreachable_warnings_are_deduplicated_per_executable_transfer() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 30));
        let returned = builder.advance(
            FlowNodeKind::Transfer(FlowTransfer::Return),
            Some(span(1, 7)),
            FlowEdgeKind::Execution,
        );
        let first = builder.fork_from(returned, Some(span(8, 9)), FlowEdgeKind::Diagnostic);
        builder.advance(
            FlowNodeKind::Transfer(FlowTransfer::Break),
            Some(span(10, 16)),
            FlowEdgeKind::Diagnostic,
        );
        builder.advance(
            FlowNodeKind::Branch,
            Some(span(17, 18)),
            FlowEdgeKind::Diagnostic,
        );
        builder.fork_from(returned, Some(span(20, 21)), FlowEdgeKind::Diagnostic);

        let graph = builder.finish(None).expect("valid diagnostic-only graph");
        let diagnostics =
            unreachable_code_diagnostics(&graph, span(0, 30)).expect("valid warning analysis");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert_eq!(diagnostics[0].code, "N3033");
        assert_eq!(diagnostics[0].labels[0].span, span(8, 9));
        assert_eq!(diagnostics[0].labels[1].span, span(1, 7));
        assert_ne!(first, returned);
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
    fn verifier_rejects_root_kind_mismatch_and_duplicate_entry() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        builder.advance(
            FlowNodeKind::Branch,
            Some(span(1, 2)),
            FlowEdgeKind::Execution,
        );
        let graph_exit = builder.cursor();
        let graph = builder.finish(Some(graph_exit)).expect("valid seed graph");

        let mut wrong_root_kind = graph.clone();
        wrong_root_kind.nodes[wrong_root_kind.entry.index()].kind = FlowNodeKind::Branch;
        let error = super::verify(&wrong_root_kind, span(0, 20))
            .expect_err("graph.entry must identify an Entry-kind node");
        assert!(error.message().contains("entry"));

        let mut duplicate_entry = graph;
        duplicate_entry.nodes[1].kind = FlowNodeKind::Entry;
        let error = super::verify(&duplicate_entry, span(0, 20))
            .expect_err("a verified CFG must contain exactly one Entry-kind node");
        assert!(error.message().contains("Entry"));
    }

    #[test]
    fn verifier_rejects_duplicate_and_out_of_order_binding_metadata() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let first = binding(0, "first", 1);
        let second = binding(1, "second", 7);
        builder.register_binding(&first);
        builder.register_binding(&second);
        let exit = builder.cursor();
        let graph = builder.finish(Some(exit)).expect("valid seed graph");

        let mut duplicate = graph.clone();
        duplicate.bindings.push(duplicate.bindings[1].clone());
        let error = super::verify(&duplicate, span(0, 20))
            .expect_err("duplicate binding identities must be rejected");
        assert!(error.message().contains("binding metadata"));

        let mut out_of_order = graph;
        out_of_order.bindings.swap(0, 1);
        let error = super::verify(&out_of_order, span(0, 20))
            .expect_err("binding metadata must remain in strict semantic identity order");
        assert!(error.message().contains("binding metadata"));
    }

    #[test]
    fn verifier_rejects_forward_edge_marked_as_backedge() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let left = builder.fork_from(entry, Some(span(1, 2)), FlowEdgeKind::Execution);
        let right = builder.fork_from(entry, Some(span(3, 4)), FlowEdgeKind::Execution);
        let join = builder.join([left, right], Some(span(5, 6)), FlowEdgeKind::Execution);
        let graph_exit = builder.cursor();
        let mut graph = builder.finish(Some(graph_exit)).expect("valid seed graph");
        graph.nodes[join.index()].predecessors[0].kind = FlowEdgeKind::Backedge;

        let error = super::verify(&graph, span(0, 20))
            .expect_err("a backedge must point to an earlier loop header");
        assert!(error.message().contains("strictly backward"));
    }

    #[test]
    fn verifier_rejects_backward_execution_edge() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let header = builder.join([entry], Some(span(1, 2)), FlowEdgeKind::Execution);
        builder.advance(
            FlowNodeKind::Branch,
            Some(span(3, 4)),
            FlowEdgeKind::Execution,
        );
        let body = builder.cursor();
        builder.add_backedge(body, header);
        let mut graph = builder.finish(None).expect("valid seed graph");
        let backedge = graph.nodes[header.index()]
            .predecessors
            .iter_mut()
            .find(|edge| edge.kind == FlowEdgeKind::Backedge)
            .expect("loop backedge");
        backedge.kind = FlowEdgeKind::Execution;

        let error = super::verify(&graph, span(0, 20))
            .expect_err("an ordinary execution edge cannot encode a backward cycle");
        assert!(error.message().contains("strictly forward"));
    }

    #[test]
    fn verifier_rejects_multiple_predecessors_on_non_join_nodes() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let value = binding(0, "value", 1);
        builder.register_binding(&value);
        builder.advance(
            FlowNodeKind::Initialize(value.id),
            Some(value.span),
            FlowEdgeKind::Execution,
        );
        builder.advance(
            FlowNodeKind::Read(value.id),
            Some(span(8, 13)),
            FlowEdgeKind::Execution,
        );
        let read = builder.cursor();
        let exit = builder.cursor();
        let mut graph = builder.finish(Some(exit)).expect("valid seed graph");
        let diagnostics = definite_initialization_diagnostics(&graph, span(0, 20))
            .expect("seed graph must verify");
        assert!(diagnostics.is_empty());

        graph.nodes[read.index()]
            .predecessors
            .push(super::FlowEdge {
                from: graph.entry,
                kind: FlowEdgeKind::Execution,
            });

        let error = super::verify(&graph, span(0, 20))
            .expect_err("only Join nodes may merge multiple predecessor paths");
        assert!(error.message().contains("non-Join"));
    }

    #[test]
    fn verifier_rejects_duplicate_join_predecessor_edges() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let left = builder.fork_from(entry, Some(span(1, 2)), FlowEdgeKind::Execution);
        let right = builder.fork_from(entry, Some(span(3, 4)), FlowEdgeKind::Execution);
        let join = builder.join([left, right], Some(span(5, 6)), FlowEdgeKind::Execution);
        let exit = builder.cursor();
        let mut graph = builder.finish(Some(exit)).expect("valid seed graph");
        let duplicate = graph.nodes[join.index()].predecessors[0];
        graph.nodes[join.index()].predecessors.push(duplicate);

        let error = super::verify(&graph, span(0, 20))
            .expect_err("verified predecessor lists must be duplicate-free");
        assert!(error.message().contains("duplicate"));
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
