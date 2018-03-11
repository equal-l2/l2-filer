extern crate rustbox;
use rustbox::{Style,RustBox,Key};
use std::process::Command;
use std::error::Error;
use std::path::Path;
use std::string::String;

fn get_dir_contents(p: &Path) -> std::io::Result<Vec<String>> {
    std::fs::read_dir(p).map(|rd| {
        vec![String::from("..")]
            .into_iter()
            .chain(rd.filter_map(|result| result.ok().and_then(|de| de.file_name().into_string().ok())))
            .collect()
    })
}

fn get_current_dir_contents() -> std::io::Result<Vec<String>> {
    get_dir_contents(std::env::current_dir().unwrap().as_path())
}

const PRINT_OFFSET:usize = 5;

struct State<'a> {
    index    : usize,
    page     : usize,
    content  : Vec<String>,
    queue    : Vec<(String,Style)>,
    error    : String,
    item_num : usize,
    rb       : &'a rustbox::RustBox
}

impl<'a> State<'a> {
    fn new(rb_ref:&rustbox::RustBox) -> State {
        State{
            index    : 0,
            page     : 0,
            content  : get_current_dir_contents().unwrap(),
            queue    : vec!(),
            error    : String::from(""),
            item_num : 0,
            rb       : rb_ref
        }
    }

    fn inc_index(&mut self){
        if self.index < self.item_num-1 { self.index += 1; };
    }

    fn dec_index(&mut self){
        if self.index > 0  { self.index -= 1; };
    }

    fn next_page(&mut self){
        let pages = self.content.len() / (self.rb.height()-PRINT_OFFSET);
        if pages != 0 && self.page < pages {
            self.index = 0;
            self.page += 1;
        }
    }

    fn prev_page(&mut self){
        if self.page > 0 {
            self.index = 0;
            self.page -= 1;
        }
    }

    fn open(&mut self){
        let s = &self.content[self.page*(self.rb.height()-PRINT_OFFSET)+self.index].clone();
        let p = Path::new(s.as_str());
        match std::fs::metadata(p) {
            Ok(v) => {
                if v.is_dir() {
                    match std::env::set_current_dir(p) {
                        Ok(_) => {
                            self.content = match get_current_dir_contents() {
                                Ok(v) => v,
                                Err(_) => {
                                    self.error = String::from("Cannot open directory");
                                    return;
                                }
                            };
                            self.index = 0;
                            self.page  = 0;
                        },
                        Err(_) => {
                            self.error = String::from("Cannot open directory");
                            return;
                        }
                    }
                }
                else {
                    let editor = std::env::var("EDITOR").unwrap_or(String::from("vi"));

                    Command::new(editor)
                        .arg(s)
                        .status()
                        .unwrap();
                }
            },
            Err(_) => { return; }
        }
    }

    fn print(&mut self){
        self.rb.clear();
        for (i, entry) in self.queue.iter().enumerate() {
            let &(ref s, ref sty) = entry;
            self.rb.print(0, i, *sty, rustbox::Color::White, rustbox::Color::Black, s.as_str());
        }
        self.rb.present();
        self.queue.clear();
        self.error = String::from("");
    }

    fn list_current_dir(&mut self) {
        let pages = self.content.len() / (self.rb.height()-PRINT_OFFSET);
        self.queue.push((std::env::current_dir().unwrap().into_os_string().into_string().unwrap(),rustbox::RB_REVERSE));
        self.queue.push((format!("Item(s): {} Page(s):{}/{}", self.content.len(), self.page+1, pages+1), rustbox::RB_REVERSE));
        self.queue.push((self.error.clone(),rustbox::RB_REVERSE));
        self.queue.push((String::from(""), rustbox::RB_NORMAL));

        let min = self.page*(self.rb.height()-PRINT_OFFSET);
        self.item_num = 0;
        for i in 0..(self.rb.height()-PRINT_OFFSET) {
            if i+min >= self.content.len() { break; }

            self.item_num += 1;
            let sty =
                if self.index == i { rustbox::RB_REVERSE }
                else               { rustbox::RB_NORMAL }
            ;
            let entry = &self.content[i+min];
            let p = Path::new(entry);
            if std::fs::metadata(p).unwrap().is_dir() {
                self.queue.push(([entry,"/"].concat(), sty));
            }
            else {
                self.queue.push((entry.to_owned(), sty));
            }
        }

        self.print();
    }
}

fn main(){
    let rb = RustBox::init(Default::default()).unwrap();
    let mut f = State::new(&rb);

    loop {
        f.list_current_dir();
        match rb.poll_event(false) {
            Ok(rustbox::Event::KeyEvent(key)) => {
                match key {
                    Key::Char('q')              => { break; }
                    Key::Down  | Key::Char('j') => { f.inc_index(); }
                    Key::Up    | Key::Char('k') => { f.dec_index(); }
                    Key::Enter                  => { f.open();      }
                    Key::Right | Key::Char('l') => { f.next_page(); }
                    Key::Left  | Key::Char('h') => { f.prev_page(); }
                    _ => { }
                }
            },
            Err(e) => panic!("{}", e.description()),
            _ => { }
        }
    }
}
