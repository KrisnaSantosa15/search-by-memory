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
    assert_eq!("cuda".parse::<Accelerator>().unwrap(), Accelerator::Cuda);
}

#[test]
fn unknown_accelerator_name_is_an_error_listing_the_choices() {
    let error = "tpu".parse::<Accelerator>().unwrap_err();

    assert!(error.contains("directml"), "{error}");
    assert!(error.contains("cuda"), "{error}");
}

#[cfg(windows)]
#[test]
fn default_order_on_windows_prefers_cuda_then_directml_then_cpu() {
    assert_eq!(
        Accelerator::default_order(),
        [Accelerator::Cuda, Accelerator::DirectMl, Accelerator::Cpu]
    );
}

#[cfg(not(windows))]
#[test]
fn default_order_elsewhere_is_cpu_only() {
    assert_eq!(Accelerator::default_order(), [Accelerator::Cpu]);
}

#[test]
fn accelerator_is_printed_by_its_command_line_name() {
    for accelerator in [
        Accelerator::Cpu,
        Accelerator::Cuda,
        Accelerator::DirectMl,
        Accelerator::DirectMlDevice(1),
    ] {
        let name = accelerator.to_string();
        assert_eq!(name.parse::<Accelerator>(), Ok(accelerator), "{name}");
    }
}
