use std::fs;
use std::process::Command;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use gtk::prelude::*;
use ksni::{Icon, MenuItem, ToolTip, Tray};

#[derive(Clone, Copy, PartialEq, Eq)]
enum CheckType {
    Http,
    Ping,
}

impl CheckType {
    fn as_str(self) -> &'static str {
        match self {
            CheckType::Http => "http",
            CheckType::Ping => "ping",
        }
    }

    fn from_str(s: &str) -> CheckType {
        match s {
            "ping" => CheckType::Ping,
            _ => CheckType::Http,
        }
    }
}

struct SiteConfig {
    url: String,
    check_type: CheckType,
    interval_secs: u64,
}

struct SiteState {
    config: SiteConfig,
    online: AtomicBool,
}

struct Config {
    sites: Vec<Arc<SiteState>>,
    timeout_secs: u64,
}

fn load_config(path: &str) -> Config {
    let content = fs::read_to_string(path).unwrap_or_default();

    struct Raw {
        url: Option<String>,
        check_type: CheckType,
        interval_secs: u64,
    }

    let mut raw_sites: std::collections::BTreeMap<u32, Raw> = std::collections::BTreeMap::new();
    let mut timeout_secs = 5u64;
    let mut default_interval_secs = 10u64;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        if key == "timeout_secs" {
            timeout_secs = value.parse().unwrap_or(timeout_secs);
            continue;
        }
        if key == "interval_secs" {
            // chiave globale legacy: usata come default per i siti senza intervallo proprio
            default_interval_secs = value.parse().unwrap_or(default_interval_secs);
            continue;
        }

        if !key.starts_with("site") {
            continue;
        }
        let rest = &key[4..];
        let Some(underscore) = rest.find('_') else {
            continue;
        };
        let Ok(index) = rest[..underscore].parse::<u32>() else {
            continue;
        };
        let field = &rest[underscore + 1..];

        let entry = raw_sites.entry(index).or_insert(Raw {
            url: None,
            check_type: CheckType::Http,
            interval_secs: default_interval_secs,
        });

        match field {
            "url" => entry.url = Some(value.to_string()),
            "type" => entry.check_type = CheckType::from_str(value),
            "interval_secs" => entry.interval_secs = value.parse().unwrap_or(default_interval_secs),
            _ => {}
        }
    }

    let mut sites: Vec<Arc<SiteState>> = raw_sites
        .into_values()
        .filter_map(|r| r.url.map(|url| (url, r.check_type, r.interval_secs)))
        .map(|(url, check_type, interval_secs)| {
            Arc::new(SiteState {
                config: SiteConfig {
                    url,
                    check_type,
                    interval_secs,
                },
                online: AtomicBool::new(true),
            })
        })
        .collect();

    if sites.is_empty() {
        sites.push(Arc::new(SiteState {
            config: SiteConfig {
                url: "https://example.com".to_string(),
                check_type: CheckType::Http,
                interval_secs: default_interval_secs,
            },
            online: AtomicBool::new(true),
        }));
    }

    Config { sites, timeout_secs }
}

fn save_config(path: &str, sites: &[(String, CheckType, u64)], timeout_secs: u64) {
    let mut content = String::new();
    for (i, (url, check_type, interval_secs)) in sites.iter().enumerate() {
        let n = i + 1;
        content.push_str(&format!("site{n}_url = {url}\n"));
        content.push_str(&format!("site{n}_type = {}\n", check_type.as_str()));
        content.push_str(&format!("site{n}_interval_secs = {interval_secs}\n"));
    }
    content.push_str(&format!("timeout_secs = {timeout_secs}\n"));
    fs::write(path, content).expect("impossibile salvare il config file");
}

fn check_http(url: &str, timeout_secs: u64) -> bool {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(timeout_secs))
        .build();

    agent.get(url).call().is_ok()
}

fn strip_to_host(input: &str) -> &str {
    let without_scheme = input
        .strip_prefix("https://")
        .or_else(|| input.strip_prefix("http://"))
        .unwrap_or(input);

    without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme)
}

fn check_ping(host: &str, timeout_secs: u64) -> bool {
    let host = strip_to_host(host);
    Command::new("ping")
        .arg("-c")
        .arg("1")
        .arg("-W")
        .arg(timeout_secs.to_string())
        .arg(host)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn check_site(config: &SiteConfig, timeout_secs: u64) -> bool {
    match config.check_type {
        CheckType::Http => check_http(&config.url, timeout_secs),
        CheckType::Ping => check_ping(&config.url, timeout_secs),
    }
}

fn load_icon(bytes: &[u8]) -> Icon {
    let img = image::load_from_memory(bytes)
        .expect("icona embedded non valida")
        .to_rgba8();
    let (width, height) = img.dimensions();

    // ksni/StatusNotifierItem vuole i pixel in ARGB big-endian.
    let mut argb = Vec::with_capacity(img.as_raw().len());
    for px in img.pixels() {
        let [r, g, b, a] = px.0;
        argb.extend_from_slice(&[a, r, g, b]);
    }

    Icon {
        width: width as i32,
        height: height as i32,
        data: argb,
    }
}

struct WatchdogTray {
    config_path: String,
    sites: Vec<Arc<SiteState>>,
    timeout_secs: Arc<AtomicU64>,
    blink_on: Arc<Mutex<bool>>,
}

impl WatchdogTray {
    fn any_offline(&self) -> bool {
        self.sites.iter().any(|s| !s.online.load(Ordering::Relaxed))
    }
}

impl Tray for WatchdogTray {
    fn id(&self) -> String {
        "watchdog-applet".into()
    }

    fn title(&self) -> String {
        "Watchdog".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let show_red = if self.any_offline() {
            *self.blink_on.lock().unwrap()
        } else {
            false
        };

        if show_red {
            vec![ICON_RED.with(|i| i.clone())]
        } else {
            vec![ICON_GREEN.with(|i| i.clone())]
        }
    }

    fn tool_tip(&self) -> ToolTip {
        let mut lines = Vec::new();
        for site in &self.sites {
            let status = if site.online.load(Ordering::Relaxed) {
                "ONLINE"
            } else {
                "OFFLINE"
            };
            lines.push(format!(
                "[{}] {} - {}",
                site.config.check_type.as_str(),
                site.config.url,
                status
            ));
        }
        ToolTip {
            title: "Watchdog".into(),
            description: lines.join("\n"),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        open_status_window(self.sites.clone());
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        use ksni::menu::*;

        let config_path = self.config_path.clone();
        let sites = self.sites.clone();
        let timeout_secs = self.timeout_secs.clone();

        vec![
            StandardItem {
                label: "Impostazioni...".into(),
                activate: Box::new(move |_| {
                    open_settings_window(config_path.clone(), sites.clone(), timeout_secs.clone());
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Esci".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

thread_local! {
    static ICON_GREEN: Icon = load_icon(include_bytes!("../icon_green.png"));
    static ICON_RED: Icon = load_icon(include_bytes!("../icon_red.png"));
}

fn open_status_window(sites: Vec<Arc<SiteState>>) {
    glib::idle_add_once(move || {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("Watchdog - Stato siti");
        window.set_default_size(360, 220);
        window.set_position(gtk::WindowPosition::CenterAlways);

        let list_box = gtk::ListBox::new();

        for site in &sites {
            let row = gtk::ListBoxRow::new();
            let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            hbox.set_margin_start(8);
            hbox.set_margin_end(8);
            hbox.set_margin_top(4);
            hbox.set_margin_bottom(4);

            let online = site.online.load(Ordering::Relaxed);
            let dot = gtk::Label::new(None);
            if online {
                dot.set_markup("<span foreground='#28c828'>\u{25cf}</span>");
            } else {
                dot.set_markup("<span foreground='#dc2828'>\u{25cf}</span>");
            }

            let label = gtk::Label::new(Some(&format!(
                "[{}] {}",
                site.config.check_type.as_str(),
                site.config.url
            )));
            label.set_halign(gtk::Align::Start);
            label.set_hexpand(true);

            hbox.pack_start(&dot, false, false, 0);
            hbox.pack_start(&label, true, true, 0);
            row.add(&hbox);
            list_box.add(&row);
        }

        window.add(&list_box);
        window.show_all();
    });
}

fn open_settings_window(
    config_path: String,
    sites: Vec<Arc<SiteState>>,
    timeout_secs: Arc<AtomicU64>,
) {
    glib::idle_add_once(move || {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("Watchdog - Impostazioni");
        window.set_default_size(520, 360);
        window.set_position(gtk::WindowPosition::CenterAlways);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
        vbox.set_margin_start(12);
        vbox.set_margin_end(12);
        vbox.set_margin_top(12);
        vbox.set_margin_bottom(12);

        let sites_label = gtk::Label::new(Some("Siti monitorati:"));
        sites_label.set_halign(gtk::Align::Start);
        vbox.pack_start(&sites_label, false, false, 0);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let h_url = gtk::Label::new(Some("URL / Host"));
        h_url.set_hexpand(true);
        h_url.set_halign(gtk::Align::Start);
        let h_type = gtk::Label::new(Some("Tipo"));
        let h_interval = gtk::Label::new(Some("Intervallo (s)"));
        header.pack_start(&h_url, true, true, 0);
        header.pack_start(&h_type, false, false, 0);
        header.pack_start(&h_interval, false, false, 0);
        header.pack_start(&gtk::Label::new(Some("      ")), false, false, 0);
        vbox.pack_start(&header, false, false, 0);

        let rows_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let rows_box_rc = Rc::new(rows_box);

        fn add_site_row(rows_box: &gtk::Box, url: &str, check_type: CheckType, interval_secs: u64) {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);

            let entry = gtk::Entry::new();
            entry.set_text(url);
            entry.set_hexpand(true);

            let type_combo = gtk::ComboBoxText::new();
            type_combo.append(Some("http"), "HTTP");
            type_combo.append(Some("ping"), "Ping");
            type_combo.set_active_id(Some(check_type.as_str()));

            let interval_spin = gtk::SpinButton::with_range(1.0, 86400.0, 1.0);
            interval_spin.set_value(interval_secs as f64);

            let remove_button = gtk::Button::with_label("Rimuovi");

            row.pack_start(&entry, true, true, 0);
            row.pack_start(&type_combo, false, false, 0);
            row.pack_start(&interval_spin, false, false, 0);
            row.pack_start(&remove_button, false, false, 0);
            rows_box.pack_start(&row, false, false, 0);
            row.show_all();

            let row_clone = row.clone();
            remove_button.connect_clicked(move |_| {
                if let Some(parent) = row_clone.parent() {
                    if let Some(container) = parent.downcast_ref::<gtk::Box>() {
                        container.remove(&row_clone);
                    }
                }
            });
        }

        for site in &sites {
            add_site_row(
                &rows_box_rc,
                &site.config.url,
                site.config.check_type,
                site.config.interval_secs,
            );
        }

        let scrolled = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scrolled.set_min_content_height(180);
        scrolled.add(&*rows_box_rc);
        vbox.pack_start(&scrolled, true, true, 0);

        let add_button = gtk::Button::with_label("+ Aggiungi sito");
        vbox.pack_start(&add_button, false, false, 0);

        {
            let rows_box_rc = rows_box_rc.clone();
            add_button.connect_clicked(move |_| {
                add_site_row(&rows_box_rc, "https://", CheckType::Http, 10);
            });
        }

        let timeout_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let timeout_label = gtk::Label::new(Some("Timeout per ogni check (secondi):"));
        let timeout_spin = gtk::SpinButton::with_range(1.0, 120.0, 1.0);
        timeout_spin.set_value(timeout_secs.load(Ordering::Relaxed) as f64);
        timeout_box.pack_start(&timeout_label, false, false, 0);
        timeout_box.pack_start(&timeout_spin, false, false, 0);
        vbox.pack_start(&timeout_box, false, false, 0);

        let note = gtk::Label::new(Some(
            "Nota: aggiungere/rimuovere siti o cambiare tipo richiede il riavvio dell'applet.",
        ));
        note.set_halign(gtk::Align::Start);
        note.style_context().add_class("dim-label");
        vbox.pack_start(&note, false, false, 0);

        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        vbox.pack_start(&separator, false, false, 4);

        let credits = gtk::Label::new(None);
        credits.set_markup(
            "<small>Server Monitor Tray \u{2014} zetappi (marcozp@gmail.com) \u{2014} 2026\n\
            <a href=\"https://www.mz-lab.eu\">www.mz-lab.eu</a></small>",
        );
        credits.set_halign(gtk::Align::Start);
        vbox.pack_start(&credits, false, false, 0);

        let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        button_box.set_halign(gtk::Align::End);
        let cancel_button = gtk::Button::with_label("Annulla");
        let save_button = gtk::Button::with_label("Salva");
        button_box.pack_start(&cancel_button, false, false, 0);
        button_box.pack_start(&save_button, false, false, 0);
        vbox.pack_start(&button_box, false, false, 0);

        window.add(&vbox);

        {
            let window = window.clone();
            cancel_button.connect_clicked(move |_| {
                window.close();
            });
        }

        {
            let window = window.clone();
            let config_path = config_path.clone();
            let rows_box_rc = rows_box_rc.clone();
            let timeout_secs = timeout_secs.clone();
            save_button.connect_clicked(move |_| {
                let mut new_sites: Vec<(String, CheckType, u64)> = Vec::new();

                for child in rows_box_rc.children() {
                    let Some(row) = child.downcast_ref::<gtk::Box>() else {
                        continue;
                    };
                    let children = row.children();
                    let (Some(entry_w), Some(combo_w), Some(spin_w)) =
                        (children.get(0), children.get(1), children.get(2))
                    else {
                        continue;
                    };
                    let Some(entry) = entry_w.downcast_ref::<gtk::Entry>() else {
                        continue;
                    };
                    let Some(combo) = combo_w.downcast_ref::<gtk::ComboBoxText>() else {
                        continue;
                    };
                    let Some(spin) = spin_w.downcast_ref::<gtk::SpinButton>() else {
                        continue;
                    };

                    let text = entry.text().to_string();
                    if text.trim().is_empty() {
                        continue;
                    }
                    let check_type = CheckType::from_str(
                        combo.active_id().map(|s| s.to_string()).unwrap_or_default().as_str(),
                    );
                    let interval = spin.value() as u64;

                    new_sites.push((text, check_type, interval));
                }

                if new_sites.is_empty() {
                    new_sites.push(("https://example.com".to_string(), CheckType::Http, 10));
                }

                let new_timeout = timeout_spin.value() as u64;
                timeout_secs.store(new_timeout, Ordering::Relaxed);

                save_config(&config_path, &new_sites, new_timeout);

                window.close();
                restart_process(&config_path);
            });
        }

        window.show_all();
    });
}

fn restart_process(config_path: &str) -> ! {
    use std::os::unix::process::CommandExt;

    let exe = std::env::current_exe().expect("impossibile determinare il percorso dell'eseguibile");
    let err = Command::new(exe).arg(config_path).exec();
    panic!("riavvio fallito: {err}");
}

fn main() {
    let config_path = std::env::args().nth(1).unwrap_or_else(|| "watchdog.conf".to_string());
    let config = load_config(&config_path);

    gtk::init().expect("impossibile inizializzare GTK (serve un display grafico)");

    let timeout_secs = Arc::new(AtomicU64::new(config.timeout_secs));
    let blink_on = Arc::new(Mutex::new(true));

    let tray = WatchdogTray {
        config_path: config_path.clone(),
        sites: config.sites.clone(),
        timeout_secs: timeout_secs.clone(),
        blink_on: blink_on.clone(),
    };

    let service = ksni::TrayService::new(tray);
    let handle = service.handle();
    service.spawn();

    // Un thread di check per sito: ognuno rispetta il proprio intervallo.
    for site in &config.sites {
        let site = site.clone();
        let handle = handle.clone();
        let timeout_secs = timeout_secs.clone();
        thread::spawn(move || loop {
            let timeout = timeout_secs.load(Ordering::Relaxed);
            let is_online = check_site(&site.config, timeout);
            let now = chrono::Local::now().format("%H:%M:%S");

            if is_online {
                println!(
                    "[{now}] \x1b[32m\u{25cf}\x1b[0m ONLINE  [{}] {}",
                    site.config.check_type.as_str(),
                    site.config.url
                );
            } else {
                println!(
                    "[{now}] \x1b[31m\u{25cf}\x1b[0m OFFLINE [{}] {}",
                    site.config.check_type.as_str(),
                    site.config.url
                );
            }

            site.online.store(is_online, Ordering::Relaxed);
            handle.update(|_| {});

            thread::sleep(Duration::from_secs(site.config.interval_secs));
        });
    }

    // Thread di lampeggio: alterna rosso/verde ogni 500ms se c'e' almeno un sito offline.
    {
        let handle = handle.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(500));
            let mut guard = blink_on.lock().unwrap();
            *guard = !*guard;
            drop(guard);
            handle.update(|_| {});
        });
    }

    gtk::main();
}
