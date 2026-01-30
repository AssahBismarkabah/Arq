use arq_core::{Phase, Task};

#[test]
fn test_new_task() {
    let task = Task::new("Add user authentication");

    assert!(!task.id.is_empty());
    assert_eq!(task.prompt, "Add user authentication");
    assert_eq!(task.phase, Phase::Research);
    assert!(task.research_doc.is_none());
    assert!(task.plan.is_none());
}

#[test]
fn test_derive_name() {
    // Task names are derived as kebab-case from the first 5 words
    let task = Task::new("Add user authentication with OAuth2 support");
    assert_eq!(task.name, "add-user-authentication-with-oauth2");

    let short_task = Task::new("Fix bug");
    assert_eq!(short_task.name, "fix-bug");
}

#[test]
fn test_to_summary() {
    let task = Task::new("Test task");
    let summary = task.to_summary();

    assert_eq!(summary.id, task.id);
    assert_eq!(summary.name, task.name);
    assert_eq!(summary.phase, task.phase);
}

#[test]
fn test_cannot_advance_without_research_doc() {
    let task = Task::new("Test task");

    // Task is in Research phase with no research_doc
    assert!(!task.can_advance());
}
