//! Theme creator dialog — GUI wrapper around shared theme_creator logic.

use adw::prelude::*;
use gtk::gio;
use soundthemed_shared::theme_creator;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub fn show_dialog(parent: &adw::ApplicationWindow) {
    let dialog = adw::AlertDialog::builder()
        .heading("Create Sound Theme")
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Theme name entry
    let name_row = adw::EntryRow::builder()
        .title("Theme Name")
        .build();

    // Folder selection: a label showing the path + a button to choose
    let folder_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();

    let folder_label = gtk::Label::builder()
        .label("No folder selected")
        .hexpand(true)
        .xalign(0.0)
        .build();

    let folder_button = gtk::Button::builder()
        .label("Choose Folder")
        .build();

    folder_button.update_property(&[gtk::accessible::Property::Label(
        "Choose source folder containing audio files",
    )]);

    folder_box.append(&folder_label);
    folder_box.append(&folder_button);

    let selected_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));

    let path_for_click = Rc::clone(&selected_path);
    let parent_for_chooser = parent.clone();
    let label_for_click = folder_label.clone();
    folder_button.connect_clicked(move |_| {
        let path_ref = Rc::clone(&path_for_click);
        let label = label_for_click.clone();
        let chooser = gtk::FileDialog::builder()
            .title("Select Source Folder")
            .build();

        chooser.select_folder(
            Some(&parent_for_chooser),
            gio::Cancellable::NONE,
            move |result: Result<gio::File, gtk::glib::Error>| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        label.set_label(&path.display().to_string());
                        *path_ref.borrow_mut() = Some(path);
                    }
                }
            },
        );
    });

    content.append(&name_row);
    content.append(&folder_box);

    dialog.set_extra_child(Some(&content));
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("create", "Create");
    dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);

    let path_for_response = Rc::clone(&selected_path);
    let parent_for_result = parent.clone();
    dialog.connect_response(None, move |_dialog, response| {
        if response != "create" {
            return;
        }

        let name = name_row.text().to_string();
        let folder = path_for_response.borrow().clone();

        if name.is_empty() {
            show_error(&parent_for_result, "Please enter a theme name.");
            return;
        }

        let folder = match folder {
            Some(f) => f,
            None => {
                show_error(&parent_for_result, "Please select a source folder.");
                return;
            }
        };

        match theme_creator::create_theme(&name, &folder) {
            Ok(result) => {
                let msg = format!(
                    "Theme '{}' created with {} sounds.\n{}",
                    name,
                    result.converted.len(),
                    if result.warnings.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "\nWarnings:\n{}",
                            result.warnings.join("\n")
                        )
                    }
                );
                show_info(&parent_for_result, &msg);
            }
            Err(e) => {
                show_error(&parent_for_result, &format!("Failed to create theme: {e}"));
            }
        }
    });

    dialog.present(Some(parent));
}

fn show_error(parent: &adw::ApplicationWindow, message: &str) {
    let dialog = adw::AlertDialog::builder()
        .heading("Error")
        .body(message)
        .build();
    dialog.add_response("ok", "OK");
    dialog.present(Some(parent));
}

fn show_info(parent: &adw::ApplicationWindow, message: &str) {
    let dialog = adw::AlertDialog::builder()
        .heading("Success")
        .body(message)
        .build();
    dialog.add_response("ok", "OK");
    dialog.present(Some(parent));
}
