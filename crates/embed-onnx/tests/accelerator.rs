use sbm_embed_onnx::Accelerator;

#[test]
fn accelerator_is_named_on_the_command_line() {
    assert_eq!("cpu".parse::<Accelerator>().unwrap(), Accelerator::Cpu);
    assert_eq!(
        "directml".parse::<Accelerator>().unwrap(),
        Accelerator::DirectMl
    );
    assert_eq!(
        "directml:1".parse::<Accelerator>().unwrap(),
        Accelerator::DirectMlDevice(1)
    );
}

#[test]
fn unknown_accelerator_name_is_an_error_listing_the_choices() {
    let error = "cuda".parse::<Accelerator>().unwrap_err();

    assert!(error.contains("directml"), "{error}");
}
