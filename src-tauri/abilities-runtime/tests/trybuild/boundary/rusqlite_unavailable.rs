use rusqlite::Connection;

fn main() {
    let _ = Connection::open_in_memory();
}
