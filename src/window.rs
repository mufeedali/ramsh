// Copyright 2022 Mufeed Ali
// SPDX-License-Identifier: GPL-3.0-or-later

use adw::subclass::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use bstr::ByteSlice;
use fastpbkdf2::pbkdf2_hmac_sha1 as pbkdf2_hmac;
use hex;
use rayon::prelude::*;
use rustc_serialize::hex::ToHex;

use std::fs::{read_to_string, File};
use std::io::BufReader;
use std::path::PathBuf;
use std::thread;
use std::time::Instant;

use crate::application::RameshApplication;
use crate::config::{APP_ID, PROFILE};

mod imp {
    use super::*;

    use gtk::CompositeTemplate;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/github/fushinari/Ramesh/ui/window.ui")]
    pub struct RameshApplicationWindow {
        pub settings: gio::Settings,
        #[template_child]
        pub main_stack: TemplateChild<adw::ViewStack>,
        // Welcome Page
        #[template_child]
        pub begin_btn: TemplateChild<gtk::Button>,
        // Network Page
        #[template_child]
        pub network_next_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub network_previous_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub network_import_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub network_essid_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub network_bssid_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub network_sta_mac_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub network_pmkid_entry: TemplateChild<adw::EntryRow>,
        // Wordlist Page
        #[template_child]
        pub wordlist_next_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub wordlist_previous_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub wordlist_import_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub wordlist_text: TemplateChild<gtk::TextView>,
        // Cracking Page
        #[template_child]
        pub cracking_progress: TemplateChild<gtk::ProgressBar>,
        // Success Page
        #[template_child]
        pub success_another_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub success_status_page: TemplateChild<adw::StatusPage>,
        // Failure Page
        #[template_child]
        pub failure_another_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub failure_status_page: TemplateChild<adw::StatusPage>,
    }

    impl Default for RameshApplicationWindow {
        fn default() -> Self {
            Self {
                settings: gio::Settings::new(APP_ID),
                main_stack: TemplateChild::default(),
                begin_btn: TemplateChild::default(),
                network_next_btn: TemplateChild::default(),
                network_previous_btn: TemplateChild::default(),
                network_import_btn: TemplateChild::default(),
                network_essid_entry: TemplateChild::default(),
                network_bssid_entry: TemplateChild::default(),
                network_sta_mac_entry: TemplateChild::default(),
                network_pmkid_entry: TemplateChild::default(),
                wordlist_next_btn: TemplateChild::default(),
                wordlist_previous_btn: TemplateChild::default(),
                wordlist_import_btn: TemplateChild::default(),
                wordlist_text: TemplateChild::default(),
                cracking_progress: TemplateChild::default(),
                success_another_btn: TemplateChild::default(),
                success_status_page: TemplateChild::default(),
                failure_another_btn: TemplateChild::default(),
                failure_status_page: TemplateChild::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RameshApplicationWindow {
        const NAME: &'static str = "RameshApplicationWindow";
        type Type = super::RameshApplicationWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RameshApplicationWindow {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            // Devel Profile
            if PROFILE == "Devel" {
                obj.add_css_class("devel");
            }

            // Load latest window state
            obj.load_window_size();
        }
    }

    impl WidgetImpl for RameshApplicationWindow {}
    impl WindowImpl for RameshApplicationWindow {
        // Save window state on delete event
        fn close_request(&self, window: &Self::Type) -> gtk::Inhibit {
            if let Err(err) = window.save_window_size() {
                log::warn!("Failed to save window state, {}", &err);
            }

            // Pass close request on to the parent
            self.parent_close_request(window)
        }
    }

    impl ApplicationWindowImpl for RameshApplicationWindow {}
    impl AdwApplicationWindowImpl for RameshApplicationWindow {}
}

glib::wrapper! {
    pub struct RameshApplicationWindow(ObjectSubclass<imp::RameshApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root;
}

impl RameshApplicationWindow {
    pub fn new(app: &RameshApplication) -> Self {
        let window: Self = glib::Object::new(&[("application", app)])
            .expect("Failed to create RameshApplicationWindow");
        window.setup_signals();
        window
    }

    fn setup_signals(&self) {
        let imp = self.imp();

        // Welcome Page
        imp.begin_btn
            .connect_clicked(clone!(@weak self as win => move |_| {
                win.page_switch("network_page");
            }));

        // Network Page
        imp.network_next_btn
            .connect_clicked(clone!(@weak self as win => move |_| {
                let imp = win.imp();
                if imp.network_essid_entry.text().is_empty() {
                    imp.network_essid_entry.add_css_class("error");
                    return;
                } else if imp.network_bssid_entry.text().is_empty() {
                    imp.network_bssid_entry.add_css_class("error");
                    return;
                } else if imp.network_sta_mac_entry.text().is_empty() {
                    imp.network_sta_mac_entry.add_css_class("error");
                    return;
                } else if imp.network_pmkid_entry.text().is_empty() {
                    imp.network_pmkid_entry.add_css_class("error");
                    return;
                }
                win.page_switch("wordlist_page");
            }));
        imp.network_previous_btn
            .connect_clicked(clone!(@weak self as win => move |_| {
                win.page_switch("welcome_page");
            }));
        imp.network_import_btn
            .connect_clicked(clone!(@weak self as win => move |_| {
                win.import_network_json();
            }));
        imp.network_essid_entry
            .connect_changed(clone!(@weak self as win => move |_| {
                let imp = win.imp();
                imp.network_essid_entry.remove_css_class("error");
            }));
        imp.network_bssid_entry
            .connect_changed(clone!(@weak self as win => move |_| {
                let imp = win.imp();
                imp.network_bssid_entry.remove_css_class("error");
            }));
        imp.network_sta_mac_entry
            .connect_changed(clone!(@weak self as win => move |_| {
                let imp = win.imp();
                imp.network_sta_mac_entry.remove_css_class("error");
            }));
        imp.network_pmkid_entry
            .connect_changed(clone!(@weak self as win => move |_| {
                let imp = win.imp();
                imp.network_pmkid_entry.remove_css_class("error");
            }));

        // Wordlist Page
        imp.wordlist_next_btn
            .connect_clicked(clone!(@weak self as win => move |_| {
                win.page_switch("cracking_page");
                win.complete_wordlist_process(None);
            }));
        imp.wordlist_previous_btn
            .connect_clicked(clone!(@weak self as win => move |_| {
                win.page_switch("network_page");
            }));
        imp.wordlist_import_btn
            .connect_clicked(clone!(@weak self as win => move |_| {
                win.import_wordlist();
            }));

        // Success Page
        imp.success_another_btn
            .connect_clicked(clone!(@weak self as win => move |_| {
                win.reset();
                win.page_switch("network_page");
            }));

        // Failure Page
        imp.failure_another_btn
            .connect_clicked(clone!(@weak self as win => move |_| {
                win.page_switch("network_page");
            }));
    }

    fn reset(&self) {
        let imp = self.imp();
        imp.network_essid_entry.set_text("");
        imp.network_bssid_entry.set_text("");
        imp.network_sta_mac_entry.set_text("");
        imp.network_pmkid_entry.set_text("");
        imp.wordlist_text.buffer().set_text("");
    }

    fn page_switch(&self, page: &str) {
        let imp = self.imp();
        imp.main_stack.set_visible_child_name(page);
    }

    fn import_network_json(&self) {
        let dialog = gtk::FileChooserNative::new(
            Some("Import Network File"),
            Some(self),
            gtk::FileChooserAction::Open,
            Some("Import"),
            Some("Cancel"),
        );
        dialog.set_modal(true);

        let json_filter = gtk::FileFilter::new();
        json_filter.add_mime_type("application/json");
        json_filter.set_name(Some("JSON"));
        dialog.add_filter(&json_filter);

        dialog.connect_response(clone!(@weak self as win => move |d, response| {
            if response == gtk::ResponseType::Accept {
                let file = &d.file().expect("Couldn't get file");

                let filename = file.path().expect("Couldn't get file path");
                let file = File::open(&filename.as_path()).expect("Couldn't open file");
                win.set_network_params(&file);
                win.page_switch("wordlist_page");
            }
            d.destroy();
        }));
        dialog.show();
    }

    fn set_network_params(&self, file: &File) {
        let imp = self.imp();

        let reader = BufReader::new(file);
        let json: serde_json::Value =
            serde_json::from_reader(reader).expect("[pmkid] file should be proper JSON!");

        let essid = json
            .get("essid")
            .unwrap()
            .as_str()
            .expect("[pmkid] config json should have essid key");

        let bssid = json
            .get("bssid")
            .unwrap()
            .as_str()
            .expect("[pmkid] config json should have bssid key");

        let sta_mac = json
            .get("sta_mac")
            .unwrap()
            .as_str()
            .expect("[pmkid] config json should have sta_mac key");

        let pmkid = json
            .get("pmkid")
            .unwrap()
            .as_str()
            .expect("[pmkid] config json should have pmkid key");

        imp.network_essid_entry.set_text(essid);
        imp.network_bssid_entry.set_text(bssid);
        imp.network_sta_mac_entry.set_text(sta_mac);
        imp.network_pmkid_entry.set_text(pmkid);
    }

    fn import_wordlist(&self) {
        let dialog = gtk::FileChooserNative::new(
            Some("Import Wordlist File"),
            Some(self),
            gtk::FileChooserAction::Open,
            Some("Import"),
            Some("Cancel"),
        );
        dialog.set_modal(true);

        let text_filter = gtk::FileFilter::new();
        text_filter.add_mime_type("text/*");
        text_filter.set_name(Some("Text Files"));
        dialog.add_filter(&text_filter);

        dialog.connect_response(clone!(@weak self as win => move |d, response| {
            if response == gtk::ResponseType::Accept {
                let file = &d.file().expect("Couldn't get file");
                let filename = file.path().expect("Couldn't get file path");

                let imp = win.imp();
                imp.main_stack.set_visible_child_name("cracking_page");

                win.complete_wordlist_process(Some(filename));
            }
            d.destroy();
        }));
        dialog.show();
    }

    fn complete_wordlist_process(&self, path: Option<PathBuf>) {
        let imp = self.imp();

        let buffer = imp.wordlist_text.buffer();
        // avoid having to load the text file into the UI if using import
        // this avoids unnecessary overhead
        let text = match path {
            None => buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), true)
                .to_string(),
            Some(file_path) => read_to_string(&file_path.as_path()).unwrap(),
        };
        let newline_split = text.lines().map(str::to_string);
        let wordlist_dict: Vec<String> = newline_split.collect();

        self.get_result(
            imp.network_essid_entry.text().to_string(),
            imp.network_bssid_entry.text().to_string(),
            imp.network_sta_mac_entry.text().to_string(),
            imp.network_pmkid_entry.text().to_string(),
            wordlist_dict,
        );
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();

        let (width, height) = self.default_size();

        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;

        imp.settings
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let imp = self.imp();

        let width = imp.settings.int("window-width");
        let height = imp.settings.int("window-height");
        let is_maximized = imp.settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    fn get_result(
        &self,
        essid: String,
        bssid: String,
        sta_mac: String,
        pmkid: String,
        wordlist_dict: Vec<String>,
    ) -> Option<&str> {
        let (sender_state, receiver_state) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let (sender_pass, receiver_pass) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let (sender_progress, receiver_progress) =
            glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let essid = essid.as_bytes().to_owned();

        let bssid = bssid
            .to_lowercase()
            .replace(":", "")
            .replace("-", "")
            .replace(".", "");

        let sta_mac = sta_mac
            .to_lowercase()
            .replace(":", "")
            .replace("-", "")
            .replace(".", "");

        let pmkid_data = pmkid
            .split("*")
            .map(str::to_string)
            .collect::<Vec<String>>();

        let pmkid_hash = &*pmkid_data.get(0).unwrap();
        let pmkid_hash = pmkid_hash.clone();

        /*
            PMKID = HMAC-SHA1-128(PMK, "PMK Name" | MAC_AP | MAC_STA)
            where the PMK is the pbkdf2_hmac of passphrase.
            params -> ("PMK Name" | MAC_AP | MAC_STA)
        */
        let params = [
            b"PMK Name",
            hex::decode(&bssid).unwrap().as_bytes(),
            hex::decode(&sta_mac).unwrap().as_bytes(),
        ]
        .concat();

        let total_crack_time = Instant::now();

        thread::spawn(move || {
            wordlist_dict.par_iter().for_each(|passphrase| {
                // returns the hash generated using the passphrase
                // compare the both pmkids and validate
                let _ = sender_progress.send(1.0 / wordlist_dict.len() as f64);

                /*
                    derive the pbkdf2 using the network name and passphrase
                    this is usually the most time consuming part
                */
                let mut key_out = [0u8; 32];
                pbkdf2_hmac(passphrase.as_bytes(), &essid, 4096, &mut key_out);

                /*
                    get the hmac-sha1 of the param using the pmk as key
                    and get the first 32 bits from its hexdigest
                */
                let hash = hmacsha1::hmac_sha1(&key_out, &params.clone().as_bytes()).to_hex();
                let pmkid = hash.get(..32);

                let new_hash = pmkid.unwrap().to_string();
                if new_hash == pmkid_hash {
                    let _ = sender_pass.send(String::from(format!(
                        "PMKID Hash: {}\nPassphrase: {}\nTime Taken: {} ms",
                        pmkid_hash,
                        passphrase,
                        total_crack_time.elapsed().as_millis()
                    )));
                };
            });
            let _ = sender_state.send(String::from("No match found"));
        });

        let imp = self.imp();

        let cracking_progress_clone = imp.cracking_progress.clone();
        receiver_progress.attach(None, move |msg| {
            cracking_progress_clone.set_fraction(cracking_progress_clone.fraction() + msg);
            glib::Continue(true)
        });

        let main_stack_clone = imp.main_stack.clone();
        let success_status_page_clone = imp.success_status_page.clone();
        receiver_pass.attach(None, move |msg| {
            success_status_page_clone.set_description(Some(msg.as_str()));
            main_stack_clone.set_visible_child_name("success_page");
            glib::Continue(true)
        });

        let main_stack_clone = imp.main_stack.clone();
        let cracking_progress_clone = imp.cracking_progress.clone();
        let failure_status_page_clone = imp.failure_status_page.clone();
        receiver_state.attach(None, move |msg| {
            failure_status_page_clone.set_description(Some(msg.as_str()));
            if main_stack_clone.visible_child_name().unwrap() != "success_page" {
                main_stack_clone.set_visible_child_name("failure_page");
            }
            cracking_progress_clone.set_fraction(0.0);
            glib::Continue(true)
        });

        return None;
    }
}
