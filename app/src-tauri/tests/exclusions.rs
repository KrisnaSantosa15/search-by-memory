//! The Folders tab's Exclusions: which are defaults, and which defaults are off.

use std::path::PathBuf;

use sbm_app::{ExclusionRow, exclusion_list};
use sbm_index::Exclusion;

#[test]
fn exclusions_are_listed_with_the_defaults_marked() {
    let mut active = Exclusion::defaults();
    active.push(Exclusion::Name("*.bak".to_string()));
    active.push(Exclusion::Folder(PathBuf::from("C:/Users/ana/private")));

    let list = exclusion_list(&active, &Exclusion::defaults());

    assert!(!list.defaults_turned_off);
    assert_eq!(
        list.rows.first(),
        Some(&ExclusionRow::Name {
            pattern: "node_modules".to_string(),
            default: true,
        })
    );
    assert_eq!(
        list.rows[list.rows.len() - 2..],
        [
            ExclusionRow::Name {
                pattern: "*.bak".to_string(),
                default: false,
            },
            ExclusionRow::Folder {
                path: PathBuf::from("C:/Users/ana/private"),
                default: false,
            },
        ]
    );
}

#[test]
fn a_default_missing_from_the_active_exclusions_is_turned_off() {
    let active: Vec<_> = Exclusion::defaults()
        .into_iter()
        .filter(|exclusion| *exclusion != Exclusion::Name("node_modules".to_string()))
        .collect();

    assert!(exclusion_list(&active, &Exclusion::defaults()).defaults_turned_off);
}

#[test]
fn a_system_drive_folder_default_is_marked_and_can_be_restored_once_off() {
    let windows = Exclusion::Folder(PathBuf::from("C:/Windows"));
    let defaults = [Exclusion::defaults(), vec![windows.clone()]].concat();

    let on = exclusion_list(&defaults, &defaults);
    let off = exclusion_list(&Exclusion::defaults(), &defaults);

    assert_eq!(
        on.rows.last(),
        Some(&ExclusionRow::Folder {
            path: PathBuf::from("C:/Windows"),
            default: true,
        })
    );
    assert!(!on.defaults_turned_off);
    assert!(off.defaults_turned_off);
}

#[test]
fn a_row_turns_back_into_the_exclusion_it_lists() {
    let name = ExclusionRow::Name {
        pattern: "*.bak".to_string(),
        default: false,
    };
    let folder = ExclusionRow::Folder {
        path: PathBuf::from("D:/old"),
        default: false,
    };

    assert_eq!(Exclusion::from(name), Exclusion::Name("*.bak".to_string()));
    assert_eq!(
        Exclusion::from(folder),
        Exclusion::Folder(PathBuf::from("D:/old"))
    );
}
