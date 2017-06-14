extern crate rustbox;
use rustbox::Style;
use rustbox::RustBox;
use std::process::{Command, Stdio};
use std::error::Error;
use std::path::Path;
use std::string::String;

fn get_dir_contents(p:&Path) -> Vec<String> {
    let mut content:Vec<_> = std::fs::read_dir(p).unwrap().map(|x| x.unwrap().path().file_name().unwrap().to_os_string().into_string().unwrap()).collect();
    content.insert(0,String::from(".."));
    content
}

fn get_current_dir_contents() -> Vec<String> {
    get_dir_contents(std::env::current_dir().unwrap().as_path())
}

struct State<'a> {
    index   :usize,
    content :Vec<String>,
    queue   :Vec<(String,Style)>,
    rb      :&'a rustbox::RustBox
}

impl<'a> State<'a> {
    fn new(rb_ref:&rustbox::RustBox) -> State {
        State{
            index:0,
            content:get_current_dir_contents(),
            queue:vec!(),
            rb:rb_ref
        }
    }

    fn inc_index(&mut self){
        if self.index < self.content.len()-1 { self.index += 1; };
    }

    fn dec_index(&mut self){
        if self.index > 0  { self.index -= 1; };
    }

    fn open(&mut self){
        let s = &self.content[self.index].clone();
        let p = Path::new(s.as_str());
        if std::fs::metadata(p).unwrap().is_dir() {
            assert!(std::env::set_current_dir(p).is_ok());
            self.index = 0;
            self.content = get_current_dir_contents();
        }
        else {
            let editor = match std::env::var("EDITOR") {
                Ok(val) => val,
                Err(_)  => String::from("vi")
            };

            Command::new(editor)
                .args(&[s])
                .status();

            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    }

    fn print(&mut self){
        for (i, entry) in self.queue.iter().enumerate() {
            let (s,sty) = entry.clone();
            self.rb.print(0,i,sty,rustbox::Color::White,rustbox::Color::Black,s.as_str());
        }
        self.rb.present();
        self.queue.clear();
    }

    fn list_current_dir(&mut self) {
        self.rb.clear();
        self.queue.push((std::env::current_dir().unwrap().into_os_string().into_string().unwrap(),rustbox::RB_REVERSE));
        self.queue.push((String::from(""),rustbox::RB_NORMAL));
        for (i, entry) in self.content.iter().enumerate() {
            let sty =
                if self.index == i { rustbox::RB_REVERSE }
                else               { rustbox::RB_NORMAL }
            ;
            let p = Path::new(entry);
            if std::fs::metadata(p).unwrap().is_dir() {
                self.queue.push(([entry,"/"].concat(),sty));
            }
            else {
                self.queue.push((entry.to_owned(),sty));
            }
        }
        self.print();
    }
}

fn main(){
    let mut rb = rustbox::RustBox::init(Default::default()).unwrap();
    let mut f = State::new(&rb);

    loop {
        f.list_current_dir();
        match rb.poll_event(false) {
            Ok(rustbox::Event::KeyEvent(key)) => {
                match key {
                    rustbox::Key::Char('q') => { break; }
                    rustbox::Key::Down |
                    rustbox::Key::Char('j') => { f.inc_index(); }
                    rustbox::Key::Up |
                    rustbox::Key::Char('k') => { f.dec_index(); }
                    rustbox::Key::Enter     => { f.open();      }
                    _ => { }
                }
            },
            Err(e) => panic!("{}", e.description()),
            _ => { }
        }
    }
}
