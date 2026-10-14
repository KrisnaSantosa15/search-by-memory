use std::io;

use sbm_core::Unreadable;

#[test]
fn file_the_system_refuses_to_open_is_reported_as_no_access() {
    let error = io::Error::from(io::ErrorKind::PermissionDenied);

    assert_eq!(Unreadable::of(&error), Unreadable::NoAccess);
}
