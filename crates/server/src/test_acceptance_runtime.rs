use acceptance_runtime::AcceptanceRuntime;

/// Loads the private acceptance handoff; explicit live runs fail closed when it is absent or bad.
pub(crate) fn load() -> AcceptanceRuntime {
    AcceptanceRuntime::load()
        .unwrap_or_else(|error| panic!("acceptance runtime is required and invalid: {error}"))
}
