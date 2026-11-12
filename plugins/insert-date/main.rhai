fn on_start() {
    ui_button("write", "Date", "insert_date");
    ui_button("write", "Time", "insert_time");
}

fn insert_date() {
    editor_insert(now_date());
}

fn insert_time() {
    let stamp = now_iso();
    // Format ISO 8601 : AAAA-MM-JJTHH:MM:SS...
    editor_insert(stamp.sub_string(11, 5));
}
