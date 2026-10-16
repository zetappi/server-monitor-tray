# Server Monitor Tray

Applet per Linux, scritto in Rust, che monitora uno o più siti/host e mostra lo stato (online/offline) tramite un'icona nella system tray.

## Funzionalità

- Monitoraggio di più siti/host in parallelo, ciascuno con intervallo di controllo indipendente
- Due tipi di controllo per sito: **HTTP** (richiesta GET) o **Ping** (ICMP via comando di sistema `ping`)
- Icona verde quando tutti i siti sono raggiungibili, icona rossa lampeggiante se almeno uno è offline
- Tooltip con l'elenco di tutti i siti e il loro stato
- Click sinistro sull'icona: finestra con lo stato dettagliato di ogni sito
- Click destro sull'icona: menu con "Impostazioni..." (aggiungi/rimuovi siti, cambia tipo di controllo e intervallo) ed "Esci"
- Al salvataggio delle impostazioni l'applet si riavvia automaticamente applicando subito la nuova configurazione

## Requisiti

- Rust (edizione 2024, toolchain stabile recente)
- GTK3 (librerie di sviluppo): su Debian/Ubuntu `sudo apt install libgtk-3-dev`
- Un desktop con supporto a system tray via StatusNotifierItem (es. GNOME con estensione tray, KDE, Cinnamon, XFCE)
- Il comando `ping` di sistema presente in `PATH` (per i controlli di tipo Ping)

## Build

```bash
cargo build --release
```

L'eseguibile sarà in `target/release/watchdog`.

## Uso

```bash
./target/release/watchdog [percorso/al/config.conf]
```

Se non specificato, viene usato `watchdog.conf` nella directory corrente. Un file di esempio è fornito in [`watchdog.conf.example`](watchdog.conf.example) — copialo in `watchdog.conf` e personalizzalo.

## Formato del file di configurazione

```ini
site1_url = https://example.com
site1_type = http
site1_interval_secs = 10

site2_url = example.com
site2_type = ping
site2_interval_secs = 15

timeout_secs = 5
```

- `siteN_url`: URL (per `http`) o host/IP (per `ping`) del sito N. Lo schema `http(s)://` viene ignorato automaticamente nei controlli ping.
- `siteN_type`: `http` oppure `ping`.
- `siteN_interval_secs`: intervallo in secondi tra due controlli di quel sito.
- `timeout_secs`: timeout (in secondi) applicato a tutti i controlli, modificabile anche a caldo dalle Impostazioni.

Il file può essere modificato anche a mano; i numeri `N` devono essere consecutivi a partire da 1.

## Licenza

MIT
