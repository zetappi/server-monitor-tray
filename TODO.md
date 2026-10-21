# TODO

Idee per sviluppi futuri, non ancora pianificate.

- [x] Notifica desktop (popup) quando un sito passa da online a offline e viceversa, via `notify-send`. (done)
- [ ] Porting Windows: fattibile ma richiede riscrivere tray icon (ksni è Linux/DBus-only), notifiche (niente notify-send), autostart (registro invece di .desktop) e i flag di `ping`. La logica di check HTTP/ping si porta quasi gratis, l'integrazione col sistema no.
- [ ] Due piccoli "LED" di attività rete (upload/download) accanto all'icona di stato: fattibile leggendo `/proc/net/dev`, ma una singola StatusNotifierItem espone una sola icona bitmap — andrebbe composta a runtime (due pallini affiancati in un'unica immagine) o usando un secondo TrayService separato.
