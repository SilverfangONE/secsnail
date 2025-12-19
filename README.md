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
