# Secure Snail Protocol 🐌 (File Transfer)
<b>by Jan Spennemann & Luis Andrés Boden</b>

## Start Demo:

Client-Demo:
````bash
cargo run --release --bin client -- --ip `[127.0.0.1]` --file-name `[FILE_NAME]` -e `[ERROR_RATE]` -l `[LOSS_RATE]` -d `[DUP_RATE]`
````

Server-Demo:
````bash
cargo run --release --bin server -- --destination `[DIR_NAME]` -e `[ERROR_RATE]` -l `[LOSS_RATE]` -d `[DUP_RATE]`
````

___
### Aufgaben

- [x] <b>a)</b> Spezifizieren Sie geeignete Zustandsautomaten für FileSender und FileReceiver.
Benennen Sie alle Zustände sinnvoll und kennzeichnen Sie gegebenenfalls Zustandsübergänge
mit den entsprechenden Ereignissen bzw. Methodenaufrufen, die zum
Zustandsübergang führen.

- [x] <b>b)</b> Spezifizieren Sie ein geeignetes Paketformat, welches alle notwendigen Elemente
enthält, um Übertragungsfehler und Paketverluste zu erkennen. Das Alternating-Bit-
Protokoll alleine ist nicht in der Lage, mit Reordering umzugehen, daher gehen wir
hier davon aus, dass kein Reordering auftritt.

- [x] <b>c)</b> Implementieren Sie FileSender und FileReceiver, indem Sie den im ersten Aufgabenteil
entwickelten Zustandsautomaten in ein Java-Programm überführen. Die im
Zustandsdiagramm verwendeten Zustände und Zustandsnamen müssen sich im Java-
Programm wiederfinden. Beachten Sie daher unbedingt auch den Einschub "Hinweise
zur Implementierung eines endlichen Zustandsautomaten" am Ende des Aufgabenblattes.
Bei der Umsetzung der Implementierung können Sie zunächst von der Annahme
ausgehen, dass keine Übertragungsfehler, Paketverluste oder Paketduplizierung auftreten.
Testen Sie ihre beiden Programme, indem Sie FileReceiver lokal auf dem
Rechner starten und dann mit FileSender eine Datei an die localhost-Adresse senden.

- [x] <b>d)</b> Gehen Sie nun davon aus, dass Übertragungsfehler auftreten können. Erweitern Sie
Ihre Implementierung so, dass Übertragungsfehler und Paketverluste erkannt und
mit Hilfe des Alternating-Bit-Protokolls behoben werden (wir gehen weiter davon
aus, dass kein Reordering auftritt). Hinweis: Falls Sie eine bestimmte, gegebene Zeit
auf den Empfang eines Paketes warten wollen, können Sie dafür beispielsweise den
aus dem vorherigen Aufgabenblatt bekannten Weg über den Socket-Timeout nutzen.
Informieren Sie sich über die entsprechenden Methoden der Java Socket-Klassen in
der Java-Dokumentation.

- [x] <b>e)</b> Zum Test Ihres Programmes implementieren Sie jetzt eine einfache Simulation eines
unzuverlässigen Kanals. Leiten Sie dazu empfangene UDP-Pakete im FileReceiver/
FileSender durch eine Filter-Klasse oder -Funktion. Diese realisiert die Simulation
eines unzuverlässigen Kanals, indem sie
  - [x] <b>e.1)</b> zufällig mit einer konfigurierbaren Wahrscheinlichkeit einen Bitfehler
im Paket verursacht
  - [x] <b>e.2)</b> zufällig mit einer konfigurierbarenWahrscheinlichkeit ein Paket verwirft
  - [x] <b>e.3)</b> zufällig mit einer konfigurierbaren Wahrscheinlichkeit ein Paket dupliziert

- [x] <b>f)</b> Testen Sie, ob Ihr Programm eine Datei der Größe 1MiB korrekt über den simulierten,
fehlerbehafteten Kanal überträgt. Parameter: ein Paket wird mit der Wahrscheinlichkeit
p=0,1 verworfen, mit der Wahrscheinlichkeit p=0,05 dupliziert und mit der
Wahrscheinlichkeit p=0,05 tritt ein Bitfehler im Paket auf. Prüfen Sie die empfangene
Datei auf korrekte Übertragung, z.B. indem Sie ein Bild übertragen oder eine
komprimierte Datei senden!

- [x] <b>g)</b> Installieren Sie FileSender und FileReceiver auf unterschiedlichen Rechnern. Welche
Zeit brauchen Sie, um die Datei zu übertragen? Welchen Goodput (in MBit/s) erzielen
Sie? Geben Sie die ermittelten Werte auf der Konsole aus.

# Notes
- Checksum: <code>CRC32</code>, <code>Adler32</code>
