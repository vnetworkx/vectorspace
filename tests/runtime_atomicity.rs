use v_langx::{Runtime, VectorType, KernelOp};

#[test]
fn failed_program_does_not_commit_earlier_operations() {
    let mut runtime = Runtime::new();
    let result = runtime.execute_ops(vec![
        KernelOp::CreateVector {
            name: "treasury".to_string(),
            vector_type: VectorType::Free,
            components: vec![100, 25, 0],
        },
        KernelOp::CreateVector {
            name: "treasury".to_string(),
            vector_type: VectorType::Free,
            components: vec![1, 0, 0],
        },
    ]);

    assert!(result.is_err());
    assert!(runtime.state.vectors.is_empty());
    assert!(runtime.state.events.is_empty());
}

#[test]
fn successful_program_commits_working_copy_once() {
    let mut runtime = Runtime::new();
    let report = runtime.execute_ops(vec![
        KernelOp::CreateVector {
            name: "treasury".to_string(),
            vector_type: VectorType::Free,
            components: vec![100, 25, 0],
        },
    ]).expect("valid program should commit");
    assert_eq!(runtime.state.vectors["treasury"].components, vec![100, 25, 0]);
    assert_eq!(report.final_state.events.len(), 1);
}
