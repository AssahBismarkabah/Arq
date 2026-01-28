use arq_core::Plan;

#[test]
fn test_plan_to_yaml() {
    let plan = Plan::new("Test Task", "Test approach");
    let yaml = plan.to_yaml().unwrap();

    assert!(yaml.contains("task_name: Test Task"));
    assert!(yaml.contains("approach: Test approach"));
}

#[test]
fn test_plan_roundtrip() {
    let plan = Plan::new("Test Task", "Test approach");
    let yaml = plan.to_yaml().unwrap();
    let loaded = Plan::from_yaml(&yaml).unwrap();

    assert_eq!(loaded.task_name, plan.task_name);
    assert_eq!(loaded.approach, plan.approach);
}
