use arq_core::ResearchDoc;

#[test]
fn test_research_doc_to_markdown() {
    let mut doc = ResearchDoc::new("Test Task");
    doc.summary = "This is a test summary".to_string();
    doc.suggested_approach = "Do the thing".to_string();

    let markdown = doc.to_markdown();

    assert!(markdown.contains("# Research: Test Task"));
    assert!(markdown.contains("This is a test summary"));
    assert!(markdown.contains("Do the thing"));
}
