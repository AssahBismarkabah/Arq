use arq_core::Phase;

#[test]
fn test_phase_progression() {
    assert_eq!(Phase::Research.next(), Some(Phase::Planning));
    assert_eq!(Phase::Planning.next(), Some(Phase::Agent));
    assert_eq!(Phase::Agent.next(), Some(Phase::Complete));
    assert_eq!(Phase::Complete.next(), None);
}

#[test]
fn test_can_advance() {
    // Research can advance (with conditions checked elsewhere)
    // Planning can advance
    // Agent can advance
    // Complete cannot advance
    assert!(Phase::Research.next().is_some());
    assert!(Phase::Planning.next().is_some());
    assert!(Phase::Agent.next().is_some());
    assert!(Phase::Complete.next().is_none());
}
