/// Recovery-aware definite-initialization join across alternative control-flow paths.
///
/// Only paths that can continue to the join point participate. A binding is definitely
/// initialized after the join exactly when every continuing path has it initialized. If no
/// path can continue, the entry fact is retained because downstream lowering is diagnostic-only.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct InitializationJoin {
    continuing_paths: usize,
    initialized_on_all_continuing_paths: bool,
}

impl InitializationJoin {
    /// Observes one path's initialization fact and whether that path reaches the join point.
    pub(crate) fn observe(&mut self, initialized: bool, continues: bool) {
        if !continues {
            return;
        }

        if self.continuing_paths == 0 {
            self.initialized_on_all_continuing_paths = initialized;
        } else {
            self.initialized_on_all_continuing_paths &= initialized;
        }
        self.continuing_paths += 1;
    }

    /// Finishes the join, using the entry fact only when every alternative is non-continuing.
    pub(crate) fn finish(self, entry_initialized: bool) -> bool {
        if self.continuing_paths == 0 {
            entry_initialized
        } else {
            self.initialized_on_all_continuing_paths
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InitializationJoin;

    #[test]
    fn no_continuing_paths_preserve_the_entry_fact() {
        for entry in [false, true] {
            let mut join = InitializationJoin::default();
            join.observe(!entry, false);
            join.observe(!entry, false);
            assert_eq!(join.finish(entry), entry);
        }
    }

    #[test]
    fn one_continuing_path_supplies_the_result_fact() {
        for initialized in [false, true] {
            let mut join = InitializationJoin::default();
            join.observe(initialized, true);
            assert_eq!(join.finish(!initialized), initialized);
        }
    }

    #[test]
    fn multiple_continuing_paths_intersect_initialization() {
        for (left, right, expected) in [
            (false, false, false),
            (false, true, false),
            (true, false, false),
            (true, true, true),
        ] {
            let mut join = InitializationJoin::default();
            join.observe(left, true);
            join.observe(right, true);
            assert_eq!(join.finish(false), expected);
        }
    }

    #[test]
    fn noncontinuing_paths_do_not_weaken_a_continuing_path() {
        let mut join = InitializationJoin::default();
        join.observe(true, true);
        join.observe(false, false);
        join.observe(false, false);
        assert!(join.finish(false));
    }
}
