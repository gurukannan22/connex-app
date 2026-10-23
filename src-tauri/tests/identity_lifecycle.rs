// ============================================================
// tests/identity_lifecycle.rs — Integration test for device identity
// ============================================================

use connex_lib::core::identity::DeviceIdentity;

#[test]
fn test_identity_lifecycle_full() {
    let temp_dir = std::env::temp_dir().join(format!("connex_test_ident_lifecycle_{}", rand::random::<u64>()));
    let _ = std::fs::remove_dir_all(&temp_dir);

    // 1. Initial creation
    let ident1 = DeviceIdentity::load_or_create(&temp_dir).expect("Initial creation must succeed");
    assert_eq!(ident1.device_id.len(), 6);
    assert_eq!(ident1.passcode.len(), 6);
    assert!(ident1.device_id.chars().all(|c| c.is_ascii_digit()));
    assert!(ident1.passcode.chars().all(|c| c.is_ascii_digit()));

    // Verify written file
    let file_path = temp_dir.join("identity.json");
    assert!(file_path.exists());
    let content = std::fs::read_to_string(&file_path).expect("Read identity file");
    assert!(content.contains(&ident1.device_id));
    assert!(content.contains(&ident1.passcode));

    // 2. Second load must return identical device_id and passcode
    let ident2 = DeviceIdentity::load_or_create(&temp_dir).expect("Second load must succeed");
    assert_eq!(ident1.device_id, ident2.device_id);
    assert_eq!(ident1.passcode, ident2.passcode);

    // 3. Rotate passcode: device_id remains immutable
    let rotated = DeviceIdentity::rotate_passcode(&temp_dir).expect("Passcode rotation must succeed");
    assert_eq!(rotated.device_id, ident1.device_id);
    assert_eq!(rotated.passcode.len(), 6);

    // 4. Subsequent load should see the updated passcode
    let ident3 = DeviceIdentity::load_or_create(&temp_dir).expect("Third load must succeed");
    assert_eq!(ident3.device_id, ident1.device_id);
    assert_eq!(ident3.passcode, rotated.passcode);

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);
}
