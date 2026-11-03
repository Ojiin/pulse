// made-up sample content
const RADAR = `<svg viewBox="0 0 18 18" width="100%" height="100%"><circle cx="9" cy="9" r="7.3" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="9" cy="9" r="4.2" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="9" cy="9" r="1.5" fill="currentColor"/></svg>`
const RADAR_SWEEP = `<svg viewBox="0 0 18 18" width="100%" height="100%"><path d="M9 9 L9 1.7 A7.3 7.3 0 0 1 15.3 5.4 Z" fill="currentColor" opacity=".35"/><circle cx="9" cy="9" r="7.3" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="9" cy="9" r="4.2" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="9" cy="9" r="1.5" fill="currentColor"/></svg>`
const D = {
  blocked: [
    { title: 'Täglicher Backup-Check', proj: 'shop-web', when: '10:27', detail: 'Restore-Test rot (Tag 2): der Fix braucht deine Freigabe, drei Bereiche sind blockiert.', type: 'Routine' },
  ],
  waiting: [
    { title: 'Preis-Watch', proj: 'shop-api', when: '11:34', detail: 'Ein neuer Anbieter wartet auf deine Entscheidung.', type: 'Routine' },
    { title: 'Newsletter-Digest', proj: 'travel-blog', when: '08:31', detail: '2 Antwortvorschläge und ein Hinweis zum Herbst-Post warten auf deine Prüfung.', type: 'Routine' },
    { title: 'Feed-Import: 24-h-Prüfung', proj: 'shop-api', when: 'gestern', detail: 'Der Plan für die dauerhaft fehlerhaften Feeds wartet auf deine Freigabe.', type: 'Routine' },
  ],
  review: [
    { title: 'Link-Check morgens', proj: 'shop-web', when: '08:32', detail: '24 Links geprüft, 2 kaputt, Ping gesendet.', type: 'Routine' },
    { title: 'Kommentare auswerten', proj: 'shop-web', when: '14:05', detail: '3 Kommentare ausgewertet, ein Vorschlag für die FAQ.', type: 'Routine' },
  ],
  running: [
    { title: 'Radar: Oberfläche entwerfen', proj: 'claude-dashboard', when: 'seit 14:58', detail: 'Chat · 2 Subagents arbeiten', type: 'Chat' },
  ],
  next: [
    { title: 'Kommentare auswerten', when: '16:05' },
    { title: 'Datenbank aufräumen', when: '20:45' },
    { title: 'Zertifikat erneuern', when: 'morgen 10:00' },
  ],
  routines: [
    { name: 'Kommentare auswerten', proj: 'shop-web', sched: 'stündlich :05, 9–22 Uhr', runs: 'iiiiiiiiwiiiii', next: '16:05' },
    { name: 'Preis-Watch', proj: 'shop-api', sched: 'werktags 11:30', runs: 'iiiwiiiiiwiw.y', next: 'morgen 11:30' },
    { name: 'Täglicher Backup-Check', proj: 'shop-web', sched: 'täglich 10:15', runs: 'wwwywwwwywwywb', next: 'morgen 10:15' },
    { name: 'Link-Check morgens', proj: 'shop-web', sched: 'werktags 08:30', runs: 'wwwwwwwwwwwwwr', next: 'morgen 08:30' },
    { name: 'Newsletter-Digest', proj: 'travel-blog', sched: 'täglich 08:30', runs: 'wwwwwwmwwwwwwy', next: 'morgen 08:30' },
    { name: 'Uptime-Bericht', proj: 'shop-web', sched: 'täglich 10:15', runs: 'wwwwwwwwwwwwwi', next: 'morgen 10:15' },
    { name: 'Wochenplanung vorbereiten', proj: 'shop-api', sched: 'montags 08:00', runs: '...........w..', next: 'Mo 08:00' },
    { name: 'Bilder-Run wöchentlich', proj: 'shop-web', sched: 'montags 11:00', runs: '...........i..', next: 'Mo 11:00' },
  ],
}
// run letters: w worked, i idle, y waiting, b blocked, r review, m missed, . none
const RUN = { w: 'worked', i: 'idle', y: 'waiting', b: 'blocked', r: 'review', m: 'missed', '.': 'none' }
const $ = (sel, html) => { document.querySelector(sel).innerHTML = html }
