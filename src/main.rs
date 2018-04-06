#[macro_use]
extern crate log;
extern crate rustbox;
extern crate simplelog;
use rustbox::{Key, RustBox, Style};
use std::process::Command;
use std::error::Error;
use std::path::Path;

/// Return a list of files
fn get_dir_contents(p: &Path) -> std::io::Result<Vec<String>> {
    let mut v: Vec<String> = std::fs::read_dir(p).map(|rd| {
        rd.filter_map(|result| {
            // Get the filename from DirEntry as OsString,
            // then convert it to String.
            // Any invalid DirEntry will be discarded
            result.ok().and_then(|de| de.file_name().into_string().ok())
        }).collect()
    })?; // Return Err if something is wrong.

    v.sort_unstable(); // Sort filenames

    Ok(std::iter::once("..".into()).chain(v.into_iter()).collect()) // Prepend ".."
}

/// Convenient wrapper for `get_dir_content`
fn get_current_dir_contents() -> std::io::Result<Vec<String>> {
    get_dir_contents(std::env::current_dir().unwrap().as_path())
}

const HEAD_LINES: usize = 5; // Lines needed for head
const FOOT_LINES: usize = 1; // Lines needed for foot

struct State<'a> {
    index: usize,
    page: usize,
    content: Vec<String>,
    head: Vec<(String, Style)>,
    body: Vec<(String, Style)>,
    foot: Vec<(String, Style)>,
    error: String,
    item_num: usize,
    rb: &'a rustbox::RustBox,
}

impl<'a> State<'a> {
    fn new(rb_ref: &rustbox::RustBox) -> State {
        State {
            index: 0,
            page: 0,
            content: get_current_dir_contents().unwrap(),
            head: vec![],
            body: vec![],
            foot: vec![],
            error: "".into(),
            item_num: 0,
            rb: rb_ref,
        }
    }

    fn inc_index(&mut self) {
        if self.index < self.item_num - 1 {
            self.index += 1;
        };
    }

    fn dec_index(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        };
    }

    fn next_page(&mut self) {
        let pages = self.get_pages_count();
        if pages != 0 && self.page < pages {
            self.index = 0;
            self.page += 1;
        }
    }

    fn prev_page(&mut self) {
        if self.page > 0 {
            self.index = 0;
            self.page -= 1;
        }
    }

    fn open(&mut self) {
        let s = &self.content[self.page * self.get_effective_height() + self.index].to_owned();
        let p = Path::new(s);
        match std::fs::metadata(p) {
            Ok(v) => {
                if v.is_dir() {
                    match std::env::set_current_dir(p) {
                        Ok(_) => {
                            self.content = match get_current_dir_contents() {
                                Ok(v) => v,
                                Err(_) => {
                                    self.error = "Cannot retrieve content of the directory".into();
                                    return;
                                }
                            };
                            self.index = 0;
                            self.page = 0;
                        }
                        Err(_) => {
                            self.error = "Cannot move to the directory".into();
                            return;
                        }
                    }
                } else {
                    // Get the appropreate editor from environmental variable `EDITOR`.
                    // If failed, it falls back to vi.
                    let editor = std::env::var("EDITOR").unwrap_or("vi".into());
                    debug!("Use {} as editor", editor);

                    match Command::new(&editor).arg(s).status() {
                        Ok(st) => {
                            debug!("Editor returned status code {}", st);
                        },
                        Err(e) => {
                            self.error = format!("Cannot execute editor {}", editor);
                            debug!("Editor execution error: {}", e);
                        }
                    }
                }
            }
            Err(_) => {
                self.error = "Cannot retrieve file metadata".into();
                return;
            }
        }
    }

    fn print_queue(&mut self) {
        let mut y = 0;
        self.rb.clear();
        for entry in self.head.iter() {
            let &(ref s, ref sty) = entry;
            self.rb.print(
                0,
                y,
                *sty,
                rustbox::Color::White,
                rustbox::Color::Black,
                s.as_str(),
            );
            y += 1;
        }

        for entry in self.body.iter() {
            let &(ref s, ref sty) = entry;
            self.rb.print(
                0,
                y,
                *sty,
                rustbox::Color::White,
                rustbox::Color::Black,
                s.as_str(),
            );
            y += 1;
        }

        y = self.rb.height() - 1;
        for entry in self.foot.iter().rev() {
            let &(ref s, ref sty) = entry;
            self.rb.print(
                0,
                y,
                *sty,
                rustbox::Color::White,
                rustbox::Color::Black,
                s.as_str(),
            );
            y -= 1;
        }

        self.rb.present();
        self.error = "".into();
    }

    fn get_pages_count(&self) -> usize {
        self.content.len() / self.get_effective_height()
    }

    fn get_effective_height(&self) -> usize {
        self.rb.height() - (HEAD_LINES + FOOT_LINES)
    }

    fn prepare_head(&mut self) {
        self.head.clear();
        let pages = self.get_pages_count();
        // Current directory
        self.head.push((
            std::env::current_dir()
                .unwrap() // The directory should have checked in `open()` already.
                .into_os_string()
                .into_string()
                .unwrap(),
            rustbox::RB_REVERSE,
        ));

        // Count for items and pages
        self.head.push((
            format!(
                "Item(s): {} Page(s):{}/{}",
                self.content.len(),
                self.page + 1,
                pages + 1
            ),
            rustbox::RB_REVERSE,
        ));

        // Error
        self.head.push((self.error.clone(), rustbox::RB_REVERSE));

        // Divider
        self.head.push(("".into(), rustbox::RB_NORMAL));
    }

    fn prepare_body(&mut self) {
        self.body.clear();
        let min = self.page * self.get_effective_height();
        self.item_num = 0;
        for i in 0..self.get_effective_height() - 1 {
            if i + min >= self.content.len() {
                // all contents is printed
                break;
            }

            self.item_num += 1;
            let sty = if self.index == i {
                rustbox::RB_REVERSE
            } else {
                rustbox::RB_NORMAL
            };
            let entry = &self.content[i + min];
            let p = Path::new(entry);
            if std::fs::metadata(p).unwrap().is_dir() {
                self.body.push(([entry, "/"].concat(), sty));
            } else {
                self.body.push((entry.to_owned(), sty));
            }
        }

        // Divider
        self.body.push(("".into(), rustbox::RB_NORMAL));
    }

    fn prepare_foot(&mut self) {
        self.foot.clear();
        self.foot.push(("FOOT TEST".into(), rustbox::RB_REVERSE));
    }

    fn print(&mut self) {
        self.rb.clear();
        self.rb.present();
        self.prepare_head();
        self.prepare_body();
        self.prepare_foot();
        self.print_queue();
    }
}

fn main() {
    let _logger = simplelog::WriteLogger::init(
        log::LevelFilter::Trace,
        simplelog::Config::default(),
        std::fs::File::create("log.txt").unwrap(),
    );

    let rb = RustBox::init(Default::default()).unwrap();
    let mut f = State::new(&rb);

    loop {
        f.print();
        match rb.poll_event(false) {
            Ok(rustbox::Event::KeyEvent(key)) => {
                debug!("Key Pressed: {:?}", key);
                match key {
                    Key::Char('q') => {
                        break;
                    }
                    Key::Down | Key::Char('j') => {
                        f.inc_index();
                    }
                    Key::Up | Key::Char('k') => {
                        f.dec_index();
                    }
                    Key::Enter => {
                        f.open();
                    }
                    Key::Right | Key::Char('l') => {
                        f.next_page();
                    }
                    Key::Left | Key::Char('h') => {
                        f.prev_page();
                    }
                    Key::Char('r') => {
                        f.print_queue();
                    }
                    _ => {}
                }
            }
            Err(e) => panic!("{}", e.description()),
            _ => {}
        }
    }
}
