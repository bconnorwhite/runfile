use std::fs;
use std::path::Path;

#[test]
fn test_all_sample_help_outputs() {
  let samples_dir = Path::new("./tests/samples");
  let expected_dir = Path::new("./tests/expected");

  // Iterate through all .runfile files in the samples directory
  for entry in fs::read_dir(samples_dir).expect("Could not read samples directory") {
    let entry = entry.expect("Invalid entry in samples directory");
    let path = entry.path();
    if path.extension().and_then(|e| e.to_str()) == Some("runfile") {
      // Extract the base name (without extension)
      let file_stem = path.file_stem().and_then(|s| s.to_str()).expect("No file stem");
      let runfile_content = fs::read_to_string(&path).expect("Could not read sample file");

      // Check the corresponding expected file exists
      let expected_file = expected_dir.join(format!("{}.txt", file_stem));
      assert!(
        expected_file.exists(),
        "Expected file {:?} does not exist for sample {:?}",
        expected_file,
        path
      );

      // Read expected output
      let expected_output = fs::read_to_string(&expected_file)
        .expect("Could not read expected file");

      // Parse the runfile and generate help output
      let runfile = run::parse_runfile(&runfile_content)
        .expect(&format!("Failed to parse runfile for sample {:?}", file_stem));

      let actual_output = runfile.generate_help(false);

      // Compare outputs
      assert_eq!(
        expected_output.trim(),
        actual_output.trim(),
        "Output mismatch for sample {:?}",
        file_stem
      );
    }
  }
}
