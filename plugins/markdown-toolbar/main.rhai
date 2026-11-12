fn on_start() {
    ui_button("write", "H1", "h1");
    ui_button("write", "B", "bold");
    ui_button("write", "I", "italic");
    ui_button("write", "</>", "code");
    ui_button("write", "Link", "link");
    ui_button("write", "Quote", "quote");
    ui_button("write", "List", "list");
    ui_button("write", "Task", "task");
}

fn h1() { editor_insert("# "); }
fn bold() { editor_insert("**bold**"); }
fn italic() { editor_insert("*italic*"); }
fn code() { editor_insert("`code`"); }
fn link() { editor_insert("[text](https://)"); }
fn quote() { editor_insert("> "); }
fn list() { editor_insert("- "); }
fn task() { editor_insert("- [ ] "); }
