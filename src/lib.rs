#![no_std]
#![feature(prelude_2024)]

use filesystem::FileSystem;
use gc_heap::CopyingHeap;
// use file_system_solution::{FileSystem, FileSystemResult};
use pc_keyboard::{DecodedKey, KeyCode};
use pluggable_interrupt_os::{println, print};
use pluggable_interrupt_os::vga_buffer::{BUFFER_WIDTH, BUFFER_HEIGHT, plot, ColorCode, Color, plot_str, is_drawable, plot_num};
use ramdisk::RamDisk;
use simple_interp::{Interpreter, InterpreterOutput, i64_into_buffer, TickResult};
// use gc_heap::CopyingHeap;

// Get rid of some spurious VSCode errors
use core::option::Option;
use core::option::Option::None;
use core::panic;
use core::prelude::rust_2024::derive;
use core::clone::Clone;
use core::cmp::{PartialEq,Eq};
use core::marker::Copy;
use core::str::from_utf8;

const FIRST_BORDER_ROW: usize = 1;
const LAST_BORDER_ROW: usize = BUFFER_HEIGHT - 1;
const TASK_MANAGER_WIDTH: usize = 10;
const TASK_MANAGER_BYTES: usize = BUFFER_HEIGHT * TASK_MANAGER_WIDTH;
const WINDOWS_WIDTH: usize = BUFFER_WIDTH - TASK_MANAGER_WIDTH;
const WINDOW_WIDTH: usize = (WINDOWS_WIDTH - 3) / 2;
const WINDOW_HEIGHT: usize = (LAST_BORDER_ROW - FIRST_BORDER_ROW - 2) / 2;
const MID_WIDTH: usize = WINDOWS_WIDTH / 2;
const MID_HEIGHT: usize = BUFFER_HEIGHT / 2;
const NUM_WINDOWS: usize = 4;

const FILENAME_PROMPT: &str = "F5 - Filename: ";

const MAX_OPEN: usize = 16;
const BLOCK_SIZE: usize = 256;
const NUM_BLOCKS: usize = 255;
const MAX_FILE_BLOCKS: usize = 8;
const MAX_FILE_BYTES: usize = MAX_FILE_BLOCKS * BLOCK_SIZE;
const MAX_FILES_STORED: usize = 30;
const MAX_FILENAME_BYTES: usize = 10;

const MAX_TOKENS: usize = 500;
const MAX_LITERAL_CHARS: usize = 30;
const STACK_DEPTH: usize = 50;
const MAX_LOCAL_VARS: usize = 20;
const HEAP_SIZE: usize = 1024;
const MAX_HEAP_BLOCKS: usize = HEAP_SIZE;

// Data type for a file system object:
// FileSystem<MAX_OPEN, BLOCK_SIZE, NUM_BLOCKS, MAX_FILE_BLOCKS, MAX_FILE_BYTES, MAX_FILES_STORED, MAX_FILENAME_BYTES>

// Data type for an interpreter object:
// Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE, MAX_HEAP_BLOCKS>>


pub struct Kernel {
    screen : [[char; BUFFER_WIDTH]; BUFFER_HEIGHT],
    process_info : [[char; TASK_MANAGER_WIDTH]; BUFFER_HEIGHT],
    file_entry : [char; BUFFER_WIDTH],
    active : usize,
    files: FileSystem<MAX_OPEN, BLOCK_SIZE, NUM_BLOCKS, MAX_FILE_BLOCKS, MAX_FILE_BYTES, MAX_FILES_STORED, MAX_FILENAME_BYTES>,
    file_count : usize,
    def_buffer : [char; MAX_FILENAME_BYTES + 1],
    q1_buffer : [char; MAX_FILENAME_BYTES + 1],
    q2_buffer : [char; MAX_FILENAME_BYTES + 1],
    q3_buffer : [char; MAX_FILENAME_BYTES + 1],
    q4_buffer : [char; MAX_FILENAME_BYTES + 1],
    buffer_offset : usize,
    editing : bool,
    new_line : bool,
    running: bool,
    waiting: bool,
    input1: [char; 20],
    input_offset1: usize,
    input_flag1 : bool,
    input2: [char; 20],
    input_offset2: usize,
    input_flag2 : bool,
    input3: [char; 20],
    input_offset3: usize,
    input_flag3 : bool,
    q1_run: (bool, bool, bool),
    q1_int : Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE, MAX_HEAP_BLOCKS>>,
    q2_run: (bool, bool, bool),
    q2_int : Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE, MAX_HEAP_BLOCKS>>,
    q3_run: (bool, bool, bool),
    q3_int : Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE, MAX_HEAP_BLOCKS>>,
    q4_run: (bool, bool, bool),
    //q4_int : Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE, MAX_HEAP_BLOCKS>>,
    ticks: [usize; 4],
    turn: [bool; 4],
    turn_index: usize,
    new_line1: bool,
    new_line2: bool,
    new_line3: bool,
    // YOUR CODE HERE
}

const HELLO: &str = r#"print("Hello, world!")"#;

const NUMS: &str = r#"print(1)
print(257)"#;

const ADD_ONE: &str = r#"x := input("Enter a number")
x := (x + 1)
print(x)"#;

const COUNTDOWN: &str = r#"count := input("count")
while (count > 0) {
    count := (count - 1)
}
print("done")
print(count)"#;

const AVERAGE: &str = r#"sum := 0
count := 0
averaging := true
while averaging {
    num := input("Enter a number:")
    if (num == "quit") {
        averaging := false
    } else {
        sum := (sum + num)
        count := (count + 1)
    }
}
print((sum / count))"#;

const PI: &str = r#"sum := 0
i := 0
neg := false
terms := input("Num terms:")
while (i < terms) {
    term := (1.0 / ((2.0 * i) + 1.0))
    if neg {
        term := -term
    }
    sum := (sum + term)
    neg := not neg
    i := (i + 1)
}
print((4 * sum))"#;



fn initial_files(disk: &mut FileSystem<MAX_OPEN, BLOCK_SIZE, NUM_BLOCKS, MAX_FILE_BLOCKS, MAX_FILE_BYTES, MAX_FILES_STORED, MAX_FILENAME_BYTES>) {
    let mut count = 0;
    for (filename, contents) in [
        ("hello", HELLO),
        ("nums", NUMS),
        ("add_one", ADD_ONE),
        ("countdown", COUNTDOWN),
        ("average", AVERAGE),
        ("pi", PI),
    ] {
    
        let fd = disk.open_create(filename).unwrap();
        disk.write(fd, contents.as_bytes()).unwrap();
        disk.close(fd);
        
        
        count += 1
    }
    
}

pub fn split_screen (mut screen : [[char; BUFFER_WIDTH]; BUFFER_HEIGHT]) -> [[char; BUFFER_WIDTH]; BUFFER_HEIGHT] {
    let mut input = [' '; BUFFER_WIDTH];
        for (i,c) in FILENAME_PROMPT.chars().enumerate(){
            input[i] = c;
        }
    for (i, c) in input.iter().enumerate() {
        screen[0][i] = *c;
    }

    //F1 header
    screen[FIRST_BORDER_ROW][WINDOW_WIDTH / 2] = 'F';
    screen[FIRST_BORDER_ROW][WINDOW_WIDTH / 2 + 1] = '1';
    //F2 header
    screen[FIRST_BORDER_ROW][MID_WIDTH + (WINDOW_WIDTH / 2)] = 'F';
    screen[FIRST_BORDER_ROW][MID_WIDTH + (WINDOW_WIDTH / 2) + 1] = '2';
    //F3 header
    screen[MID_HEIGHT][WINDOW_WIDTH / 2] = 'F';
    screen[MID_HEIGHT][WINDOW_WIDTH / 2 + 1] = '3';
    //F4 header
    screen[MID_HEIGHT][MID_WIDTH + (WINDOW_WIDTH / 2)] = 'F';
    screen[MID_HEIGHT][MID_WIDTH + (WINDOW_WIDTH / 2) + 1] = '4';

    screen = update_screen(screen, 1);

    return screen
    

}

pub fn update_screen(mut screen: [[char; BUFFER_WIDTH]; BUFFER_HEIGHT], num: usize) -> [[char; BUFFER_WIDTH]; BUFFER_HEIGHT] {
    for i in 0..BUFFER_HEIGHT {
        for j in 0..BUFFER_WIDTH{
            if (i == FIRST_BORDER_ROW || i == MID_HEIGHT || i == LAST_BORDER_ROW) && j <= WINDOWS_WIDTH && !screen[i][j].is_alphanumeric(){ //top,middle,bottom row
                screen[i][j] = '.';

            } else if (j == 0 || j == MID_WIDTH || j == WINDOWS_WIDTH) && i > 0  && !screen[i][j].is_alphanumeric(){ //left, middle, right sides
                screen[i][j] = '.';
            }
        }
    }

    for i in 0..BUFFER_HEIGHT {
        for j in 0..BUFFER_WIDTH {
            if num == 1{
                if (i == FIRST_BORDER_ROW || i == MID_HEIGHT) && j <= MID_WIDTH && screen[i][j] == '.' {
                    screen[i][j] = '*'
                } else if (j == 0 || j == MID_WIDTH) && i <  MID_HEIGHT  && screen[i][j] == '.' {
                    screen[i][j] = '*'
                }
            }else if num == 2{
                if (i == FIRST_BORDER_ROW || i == MID_HEIGHT) && j >= MID_WIDTH && screen[i][j] == '.' {
                    screen[i][j] = '*'
                } else if (j == MID_WIDTH || j == WINDOWS_WIDTH) && i < MID_HEIGHT  && screen[i][j] == '.' {
                    screen[i][j] = '*'
                }
            } else if num == 3{
                if (i == MID_HEIGHT || i == LAST_BORDER_ROW) && j <= MID_WIDTH && screen[i][j] == '.' {
                    screen[i][j] = '*'
                } else if (j == 0 || j == MID_WIDTH) && i >= MID_HEIGHT  && screen[i][j] == '.' {
                    screen[i][j] = '*'
                }
            } else if num == 4{
                if (i == MID_HEIGHT || i == LAST_BORDER_ROW) && j >= MID_WIDTH && screen[i][j] == '.' {
                    screen[i][j] = '*'
                } else if (j == MID_WIDTH || j == WINDOWS_WIDTH) && i >= MID_HEIGHT  && screen[i][j] == '.' {
                    screen[i][j] = '*'
                }
            }
        }
    }

    return screen
}

impl Kernel {
    pub fn new() -> Self {
        let mut screen = [[' '; BUFFER_WIDTH]; BUFFER_HEIGHT];
        let mut files: FileSystem<MAX_OPEN, BLOCK_SIZE, NUM_BLOCKS, MAX_FILE_BLOCKS, MAX_FILE_BYTES, MAX_FILES_STORED, MAX_FILENAME_BYTES> = filesystem::FileSystem::new(RamDisk::new()); 
        let process_info= [[' '; TASK_MANAGER_WIDTH]; BUFFER_HEIGHT];
        let active = 1;
        let file_count = 0;
        initial_files(&mut files);
        screen = split_screen(screen);
        let file_entry = screen[0];
        let def_buffer = [' '; MAX_FILENAME_BYTES + 1];
        let q1_buffer = [' '; MAX_FILENAME_BYTES + 1];
        let q2_buffer = [' '; MAX_FILENAME_BYTES + 1];
        let q3_buffer = [' '; MAX_FILENAME_BYTES + 1];
        let q4_buffer = [' '; MAX_FILENAME_BYTES + 1];
        let buffer_offset = 0;
        let editing = false;
        let new_line = false;
        let running = false;
        let waiting = false;
        let input1 = ['\0'; 20];
        let input_offset1 = 0;
        let input_flag1 = false;
        let input2 = ['\0'; 20];
        let input_offset2 = 0;
        let input_flag2 = false;
        let input3 = ['\0'; 20];
        let input_offset3 = 0;
        let input_flag3 = false;
        let q1_run = (false, false, false);
        let q1_int = Interpreter::new("");
        let q2_run = (false, false, false);
        let q2_int = Interpreter::new("");
        let q3_run = (false, false, false);
        let q3_int = Interpreter::new("");
        let q4_run = (false, false, false);
        let ticks = [0; 4];
        let turn = [false; 4];
        let turn_index = 1;
        let new_line1 = false;
        let new_line2 = false;
        let new_line3= false;
        //let q4_int = Interpreter::new("");

        Self{screen, process_info, file_entry, active, files, file_count, q1_buffer ,q2_buffer,q3_buffer,q4_buffer, buffer_offset,def_buffer, editing, new_line, running, waiting, input1, input_offset1, q1_run, q1_int, input_flag1, q2_run, q2_int, q3_run, q3_int, q4_run, ticks , turn, turn_index, new_line1, new_line2, new_line3, input2, input_offset2, input_flag2, input3, input_offset3, input_flag3 } //,q4_int}
        //todo!("Create your kernel object");
    }

    pub fn key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(code) => self.handle_raw(code),
            DecodedKey::Unicode(c) => self.handle_unicode(c)
        }
        self.draw();
    }
    
    fn update_active(&mut self, num: usize) {
        
        if self.active != num && !self.editing{
            self.active = num;
            self.reset_buffers();
            self.buffer_offset = 0;
            self.screen = update_screen(self.screen, num);
            
        }
        
    }
    
    fn reset_buffers(&mut self) {
        self.q1_buffer = self.def_buffer;
        self.q2_buffer = self.def_buffer;
        self.q3_buffer = self.def_buffer;
        self.q4_buffer = self.def_buffer;
    }
    
    fn add_files(&mut self, editing: bool ) {
        let directory = self.files.list_directory().unwrap();
        let file_count = directory.0;
        let filenames = directory.1;

        if editing || file_count != self.file_count {
            self.file_count = file_count;  
            let col_width = WINDOW_WIDTH / 3;
            let mut word_count = 0;
            for i in FIRST_BORDER_ROW+1..WINDOW_HEIGHT {
                let mut count = 1;
                for j in 1..WINDOW_WIDTH{
                    if count < col_width {
                        if word_count < 1 {
                            self.def_buffer[count - 1] = filenames[word_count][count - 1] as char;
                            self.q1_buffer[count - 1] = filenames[word_count][count - 1] as char;
                            self.q2_buffer[count - 1] = filenames[word_count][count - 1] as char;
                            self.q3_buffer[count - 1] = filenames[word_count][count - 1] as char;
                            self.q4_buffer[count - 1] = filenames[word_count][count - 1] as char;
                        }
                        self.screen[i][j] = filenames[word_count][count - 1] as char;
                        self.screen[i][j + WINDOW_WIDTH + 2] = filenames[word_count][count - 1] as char;
                        self.screen[i + WINDOW_HEIGHT+ 1][j] = filenames[word_count][count - 1] as char;
                        self.screen[i + WINDOW_HEIGHT + 1][j + WINDOW_WIDTH + 2] = filenames[word_count][count - 1] as char;
                        count += 1;
                    } else if count == col_width{
                        word_count += 1;
                        count = 1;
                    }
                }
                word_count += 1;
        }
    }
}

    fn handle_raw(&mut self, key: KeyCode) {
        match key {
            KeyCode::F1=> {
                self.update_active(1)
            }
            KeyCode::F2=> {
                self.update_active(2)
            }
            KeyCode::F3=> {
                self.update_active(3)
            }
            KeyCode::F4=> {
                self.update_active(4)
            }
            KeyCode::F5=> {
                self.update_active(5);
            } 
            KeyCode::F6=> {
                self.buffer_offset = 0;
                if self.editing {
                    let buffer = self.empty_screen();
                    let name_buff = self.clear_editing();
                    let filename = from_utf8(&name_buff).unwrap();
                    // println!("{:?}", buffer);
                    // panic!();
                    self.screen = update_screen(self.screen, self.active);
                    self.add_files(true);
                    self.editing = false;
                    let fd = self.files.open_create(filename).unwrap();
                    self.files.write(fd, &buffer);
                    self.files.close(fd);
                } else if self.active == 1 {
                    self.q1_run = (false, false, false);
                    self.waiting_check();    
                    self.input1 = ['\0' ; 20];
                    self.input_flag1 = false;
                    self.input_offset1 = 0;
                    self.empty_screen();
                    self.screen = update_screen(self.screen, 1);
                    self.add_files(true); 
                    self.ticks[0] = 0
                } else if self.active == 2 {
                    self.q2_run = (false, false, false);
                    self.waiting_check();    
                    self.input2 = ['\0' ; 20];
                    self.input_flag2 = false;
                    self.input_offset2 = 0;
                    self.empty_screen();
                    self.screen = update_screen(self.screen, 2);
                    self.add_files(true); 
                    self.ticks[1] = 0
                }else if self.active == 3 {
                    self.q3_run = (false, false, false);
                    self.waiting_check();    
                    self.input3 = ['\0' ; 20];
                    self.input_flag3 = false;
                    self.input_offset3 = 0;
                    self.empty_screen();
                    self.screen = update_screen(self.screen, 3);
                    self.add_files(true); 
                    self.ticks[2] = 0
                }
                else if self.active == 4 {
                    self.q4_run = (false, false, false);
                    self.waiting_check();    
                    self.input1 = ['\0' ; 20];
                    self.input_flag1 = false;
                    self.input_offset1 = 0;
                    self.empty_screen();
                    self.screen = update_screen(self.screen, 4);
                    self.add_files(true); 
                    self.ticks[3] = 0
                }
                self.draw();
            } 
            KeyCode::ArrowRight => {
                self.highlight('r');
            }
            KeyCode::ArrowLeft => {
                self.highlight('l');
            }
            KeyCode::ArrowDown => {
                self.highlight('d');
            }
            KeyCode::ArrowUp => {
                self.highlight('u');
            }
            
            _ => ()
            }
        }
            
    fn create_file(&mut self) {

        let mut buffer = [0; MAX_FILENAME_BYTES];
        let start = FILENAME_PROMPT.len();
        let mut count = 0;
        for i in start..start+MAX_FILENAME_BYTES {
            buffer[count] = self.screen[0][i] as u8;
            self.screen[0][i] = ' ';
            count += 1;
        }
        let filename = from_utf8(&buffer).unwrap();
        let fd = self.files.open_create(filename).unwrap();
        self.files.close(fd).unwrap();
    }

    fn empty_screen(&mut self) -> [u8; 450]  {
        let mut buffer: [u8; 450] = [0; 450];
        let mut count = 0;
        if self.active == 1 {
            for i in FIRST_BORDER_ROW + 1..MID_HEIGHT {
                for j in 1..MID_WIDTH {
                    buffer[count] = self.screen[i][j] as u8;
                    self.screen[i][j] = ' ';
                    count += 1;
                }
            }
        } else if self.active == 2 {
            for i in FIRST_BORDER_ROW + 1..MID_HEIGHT {
                for j in MID_WIDTH + 1..WINDOWS_WIDTH {
                    buffer[count] = self.screen[i][j] as u8;
                    self.screen[i][j] = ' ';
                    count += 1;
                }
            }
        } else if self.active == 3 {
            for i in MID_HEIGHT + 1..LAST_BORDER_ROW {
                for j in 1..MID_WIDTH {
                    buffer[count] = self.screen[i][j] as u8;
                    self.screen[i][j] = ' ';
                    count += 1;
                }
            }
        } else if self.active == 4 {
            for i in MID_HEIGHT + 1..LAST_BORDER_ROW {
                for j in MID_WIDTH + 1..WINDOWS_WIDTH {
                    buffer[count] = self.screen[i][j] as u8;
                    self.screen[i][j] = ' ';
                    count += 1;
                }
            }
        
        }
        return buffer;
    }

    fn clear_editing(&mut self) -> [u8; MAX_FILENAME_BYTES] {
        let mut name = [0; MAX_FILENAME_BYTES];
        if self.active == 1 {
            self.screen[FIRST_BORDER_ROW][2] = '*';
            self.screen[FIRST_BORDER_ROW][3] = '*';
            self.screen[FIRST_BORDER_ROW][4] = '*';
            self.screen[FIRST_BORDER_ROW][5] = '*';
            let mut len = 0;
            
            for i in self.q1_buffer {
                if i != '\0'{
                    if len == MAX_FILENAME_BYTES {

                    } else{
                        name[len] = i as u8;
                    }
                    len += 1
                } else {
                    break;
                }
            }
            for i in 0..len{
                self.screen[FIRST_BORDER_ROW][6 + i] = '*';
                plot('*', 6 + i, FIRST_BORDER_ROW , ColorCode::new(Color::Black, Color::White));
            }
            
        } else if self.active == 2 {
            self.screen[FIRST_BORDER_ROW][MID_WIDTH + 2] = '*';
            self.screen[FIRST_BORDER_ROW][MID_WIDTH + 3] = '*';
            self.screen[FIRST_BORDER_ROW][MID_WIDTH + 4] = '*';
            self.screen[FIRST_BORDER_ROW][MID_WIDTH + 5] = '*';
            let mut len = 0;
            for i in self.q2_buffer {
                if i != '\0'{
                    name[len] = i as u8;
                    len += 1
                } else {
                    break;
                }
            }
            for i in 0..len{
                self.screen[FIRST_BORDER_ROW][MID_WIDTH + 6 + i] = '*';
                plot('*', MID_WIDTH + 6 + i, FIRST_BORDER_ROW , ColorCode::new(Color::Black, Color::White));
            }
        } else if self.active == 3 {
            self.screen[MID_HEIGHT][2] = '*';
            self.screen[MID_HEIGHT][3] = '*';
            self.screen[MID_HEIGHT][4] = '*';
            self.screen[MID_HEIGHT][5] = '*';
            let mut len = 0;
            for i in self.q3_buffer {
                if i != '\0'{
                    name[len] = i as u8;
                    len += 1
                } else {
                    break;
                }
            }
            for i in 0..len{
                self.screen[MID_HEIGHT][6 + i] = '*';
                plot('*', 6 + i, MID_HEIGHT , ColorCode::new(Color::Black, Color::White));
            }
        } else if self.active == 4 {
            self.screen[MID_HEIGHT][MID_WIDTH + 2] = '*';
            self.screen[MID_HEIGHT][MID_WIDTH + 3] = '*';
            self.screen[MID_HEIGHT][MID_WIDTH + 4] = '*';
            self.screen[MID_HEIGHT][MID_WIDTH + 5] = '*';
            let mut len = 0;
            for i in self.q4_buffer {
                if i != '\0'{
                    name[len] = i as u8;
                    len += 1
                } else {
                    break;
                }
            }
            for i in 0..len{
                self.screen[MID_HEIGHT][MID_WIDTH + 6 + i] = '*';
                plot('*', MID_WIDTH + 6 + i, MID_HEIGHT , ColorCode::new(Color::Black, Color::White));
            }
        }
        return name;
    }
    
    fn setup_editing_window(&mut self) {
        self.editing = true;
        if self.active == 1 {
            self.screen[FIRST_BORDER_ROW][2] = '(';
            self.screen[FIRST_BORDER_ROW][3] = 'F';
            self.screen[FIRST_BORDER_ROW][4] = '6';
            self.screen[FIRST_BORDER_ROW][5] = ')';
            let mut len = 0;
            for i in self.q1_buffer {
                if i != '\0'{
                    len += 1
                } else {
                    break;
                }
            }
            for i in 0..len{
                self.screen[FIRST_BORDER_ROW][6 + i] = self.q1_buffer[i];
                plot(self.q1_buffer[i], 6 + i, FIRST_BORDER_ROW , ColorCode::new(Color::Black, Color::White));
            }
            
        }else if self.active == 2 {
            self.screen[FIRST_BORDER_ROW][MID_WIDTH + 2] = '(';
            self.screen[FIRST_BORDER_ROW][MID_WIDTH + 3] = 'F';
            self.screen[FIRST_BORDER_ROW][MID_WIDTH + 4] = '6';
            self.screen[FIRST_BORDER_ROW][MID_WIDTH + 5] = ')';
            let mut len = 0;
            for i in self.q2_buffer {
                if i != '\0'{
                    len += 1
                } else {
                    break;
                }
            }
            for i in 0..len{
                self.screen[FIRST_BORDER_ROW][MID_WIDTH + 6 + i] = self.q2_buffer[i];
                plot(self.q2_buffer[i], MID_WIDTH + 6 + i, FIRST_BORDER_ROW , ColorCode::new(Color::Black, Color::White));
            } 
        } else if self.active == 3 {
            self.screen[MID_HEIGHT][2] = '(';
            self.screen[MID_HEIGHT][3] = 'F';
            self.screen[MID_HEIGHT][4] = '6';
            self.screen[MID_HEIGHT][5] = ')';
            let mut len = 0;
            for i in self.q3_buffer {
                if i != '\0'{
                    len += 1
                } else {
                    break;
                }
            }
            for i in 0..len{
                self.screen[MID_HEIGHT][6 + i] = self.q3_buffer[i];
                plot(self.q3_buffer[i], 6 + i, MID_HEIGHT , ColorCode::new(Color::Black, Color::White));
            } 
        } else if self.active == 4 {
            self.screen[MID_HEIGHT][MID_WIDTH + 2] = '(';
            self.screen[MID_HEIGHT][MID_WIDTH + 3] = 'F';
            self.screen[MID_HEIGHT][MID_WIDTH + 4] = '6';
            self.screen[MID_HEIGHT][MID_WIDTH + 5] = ')';
            let mut len = 0;
            for i in self.q4_buffer {
                if i != '\0'{
                    len += 1
                } else {
                    break;
                }
            }
            for i in 0..len{
                self.screen[MID_HEIGHT][MID_WIDTH + 6 + i] = self.q4_buffer[i];
                plot(self.q4_buffer[i], MID_WIDTH + 6 + i, MID_HEIGHT , ColorCode::new(Color::Black, Color::White));
            } 
        }
    } 

    fn read_file_to_window(&mut self) {
        let mut buffer = [0; MAX_FILENAME_BYTES];
        if self.active == 1 {
            for (i,c) in self.q1_buffer.iter().enumerate() {
                if i == MAX_FILENAME_BYTES {
                    break;
                }
                buffer[i] = *c as u8;
            }
        } else if self.active == 2 {
            for (i,c) in self.q2_buffer.iter().enumerate() {
                if i == MAX_FILENAME_BYTES {
                    break;
                }
                buffer[i] = *c as u8;
            }
        } else if self.active == 3 {
            for (i,c) in self.q3_buffer.iter().enumerate() {
                if i == MAX_FILENAME_BYTES {
                    break;
                }
                buffer[i] = *c as u8;
            }
        } else if self.active == 4 {
            for (i,c) in self.q4_buffer.iter().enumerate() {
                if i == MAX_FILENAME_BYTES {
                    break;
                }
                buffer[i] = *c as u8;
            }
        }
        
        let filename = from_utf8(&buffer).unwrap();
        let fd = self.files.open_read(filename).unwrap();
        let mut count = 0;
        let mut file = ['\0' ; 10000];
        let mut buffer = [0;10];
        
        loop{
            let num_bytes = self.files.read(fd, &mut buffer).unwrap();
            let s = core::str::from_utf8(&buffer[0..num_bytes]).unwrap();
            for c in s.chars() {
                file[count] = c;
                
                count += 1;
            }
            if num_bytes < buffer.len() {
                self.files.close(fd);
                break;
            }
        }

        self.empty_screen();
        self.setup_editing_window();
        let mut offset_row = 0;
        let mut offset_col = 0;
        let mut col_reset = 0;
        if self.active == 1 {
            offset_row = 2;
            offset_col = 1;
            col_reset = 1;
        } else if self.active == 2 {
            offset_row = 2;
            offset_col = MID_WIDTH + 1;
            col_reset = MID_WIDTH + 1;
        } else if self.active == 3 {
            offset_row = MID_HEIGHT + 1;
            offset_col = 1;
            col_reset = 1;
        } else if self.active == 4 {
            offset_row = MID_HEIGHT + 1;
            offset_col = MID_WIDTH + 1;
            col_reset = MID_WIDTH + 1;
        }

        for (i, c) in file.iter().enumerate() {
            if i == count {
                break;
            }
            if offset_row % MID_HEIGHT == 0{
                break;
            }
            if offset_col % MID_WIDTH == 0 {
                offset_row += 1;
                offset_col = col_reset;
            }
            if *c == '\n' {
                offset_row += 1;
                offset_col = col_reset;
            } else {
                self.screen[offset_row][offset_col] = *c;
                offset_col += 1;
            }
        }
    }

    fn edit(&mut self, key : char, active: usize) {
        let mut last_char = false;
        let mut spot = (0,0);
        if active == 1 {
            for i in FIRST_BORDER_ROW + 1 .. MID_HEIGHT {
                for j in 1..MID_WIDTH{
                    if self.screen[i][j] == ' ' && !last_char {
                        spot = (i, j); 
                        last_char = true;
                    }
                    if self.screen[i][j] != ' ' && last_char {
                        last_char = false;
                    }
                }
            }
        } else if active == 2 {
            for i in FIRST_BORDER_ROW + 1 .. MID_HEIGHT {
                for j in MID_WIDTH + 1..WINDOWS_WIDTH{
                    if self.screen[i][j] == ' ' && !last_char {
                        spot = (i, j); 
                        last_char = true;
                    }
                    if self.screen[i][j] != ' ' && last_char {
                        last_char = false;
                    }
                }
            }
        } else if active == 3 {
            for i in MID_HEIGHT + 1 .. LAST_BORDER_ROW {
                for j in 1..MID_WIDTH{
                    if self.screen[i][j] == ' ' && !last_char {
                        spot = (i, j); 
                        last_char = true;
                    }
                    if self.screen[i][j] != ' ' && last_char {
                        last_char = false;
                    }
                }
            }
        } else if active == 4 {
            for i in MID_HEIGHT + 1 .. LAST_BORDER_ROW {
                for j in MID_WIDTH + 1..WINDOWS_WIDTH{
                    if self.screen[i][j] == ' ' && !last_char {
                        spot = (i, j); 
                        last_char = true;
                    }
                    if self.screen[i][j] != ' ' && last_char {
                        last_char = false;
                    }
                }
            }
        }     

        if self.new_line1 && active == 1{
            self.new_line1 = false;
            spot.0 = spot.0 + 1;
            spot.1 = 1;
        } else if self.new_line2 && active == 2 {
            self.new_line2 = false;
            spot.0 = spot.0 + 1;
            spot.1 = MID_WIDTH + 1;
        } else if self.new_line3 && active == 3{
            self.new_line3 = false;
            spot.0 = spot.0 + 1;
            spot.1 = 1;
        }
            // if self.new_line {
            //     self.new_line = false;
            //     spot.0 = spot.0 + 1;
            //     if self.active == 1 || self.active == 3 {
            //         spot.1 = 1;
            //     } else {
            //         spot.1 = MID_WIDTH + 1;
            //     }
               
            // }
            
            
            

            if key == '\u{08}' {
                if spot.1 <= 1 {
                    self.screen[spot.0 - 1][MID_WIDTH - 1] = ' ';
                } else  {
                    self.screen[spot.0][spot.1 - 1] = ' ';
                }          
            } else if key == '\n' {
                if spot.0 + 1 == MID_HEIGHT {
                    //Do something with scrolling?
                } else  {
                    self.screen[spot.0 + 1][1] = ' ';
                    self.new_line = true;
                    if active == 1 {
                        self.new_line1 = true;
                    } else if active == 2{
                        self.new_line2 = true;
                    } else if active == 3{
                        self.new_line3 = true;
                    }
                }
            }
            else {
                self.screen[spot.0][spot.1] = key;
            }
            
            
    }

    fn run(&mut self) {
        let mut buffer = [0; MAX_FILENAME_BYTES];
        if self.active == 1 {   
            if !self.q1_run.0 {
                if !self.q1_run.1 {
                    self.empty_screen();
                }
                for i in 0..MAX_FILENAME_BYTES {
                    buffer[i] = self.q1_buffer[i] as u8;
                }
            }  
        } else if self.active == 2 {
            if !self.q2_run.0 {
                if !self.q2_run.1 {
                    self.empty_screen();
                }
                for i in 0..MAX_FILENAME_BYTES {
                    buffer[i] = self.q2_buffer[i] as u8;
                }
            }
        } else if self.active == 3{
            if !self.q3_run.0 {
                if !self.q3_run.1 {
                    self.empty_screen();
                }
                for i in 0..MAX_FILENAME_BYTES {
                    buffer[i] = self.q3_buffer[i] as u8;
                }
            }
        } else if self.active == 4{
            if !self.q4_run.0 {
                if !self.waiting {
                    self.empty_screen();
                }
                for i in 0..MAX_FILENAME_BYTES {
                    buffer[i] = self.q4_buffer[i] as u8;
                }
            }
        }
            
        let filename = from_utf8(&buffer).unwrap();
        let fd = self.files.open_read(filename).unwrap();
        let mut count = 0;
        let mut file = [0 ; 10000];
        let mut buffer = [0;10];
        
        loop{
            let num_bytes = self.files.read(fd, &mut buffer).unwrap();
            let s = core::str::from_utf8(&buffer[0..num_bytes]).unwrap();
            for c in s.chars() {
                file[count] = c as u8;
                
                count += 1;
            }
            if num_bytes < buffer.len() {
                self.files.close(fd);
                break;
            }
        }
            let program = core::str::from_utf8(&file[0..count]).unwrap();
            if self.active == 1 {   
                if !self.q1_run.0 { 
                    self.q1_int = Interpreter::new(program);
                    self.q1_run.0 = true;
                    self.q1_run.2 = true;
                    self.turn[0] = true;
                }  
            } else if self.active == 2 {
                if !self.q2_run.0 {
                    self.q2_int = Interpreter::new(program);
                    self.q2_run.0 = true;
                    self.q2_run.2 = true;
                    self.turn[1] = true;
                }
            } else if self.active == 3 {
                if !self.q3_run.0 {
                    self.q3_int = Interpreter::new(program);
                    self.q3_run.0 = true;
                    self.q3_run.2 = true;
                    self.turn[2] = true;
                }
            } else if self.active == 4 {
                if !self.q4_run.0 {
                    //self.q4_int = Interpreter::new(program);
                    self.q4_run.0 = true;
                    self.turn[3] = true;
                }
            }
    }

    pub fn run_one_instruction(&mut self) {
        if self.q1_run.0 || self.q2_run.0 || self.q3_run.0 || self.q4_run.0 {
            if self.q1_run.0 && self.turn_index == 1 {
                if !self.q1_run.1 { //if screen 1 is waiting
                    if self.input_flag1 {
                        self.q1_int.provide_input(&self.input1[0..self.input_offset1]);
                        self.input_offset1 = 0;
                        self.input_flag1 = false;
                    } 
                    let mut output = KernelOutput::new(1, self.screen, self.new_line1);
                    let result: TickResult<()> = self.q1_int.tick(&mut output);
                    self.ticks[self.turn_index - 1] += 1;
                    let tmp = output.return_values();
                    self.screen = tmp.1;
                    self.new_line1 = tmp.2;
                    
                    match result {
                        
                        TickResult::Ok(()) => {
                        },
                        TickResult::Finished => {
                            let s = ['[', 'D', 'O', 'N', 'E', ']'];
                            for c in s {
                                self.edit(c, 1);
                            }
                            //self.draw();
                            self.q1_run = (false, false, true);
                        } ,
                        TickResult::AwaitInput => {
                            self.waiting = true;
                            self.q1_run.1 = true;
                        },
                        TickResult::Err(e) => {
                            println!("{:?}", e);
                            panic!()
                        },
                }
                self.draw();
            }    
            } 
            if self.q2_run.0 && self.turn_index == 2 {
                if !self.q2_run.1 { //if screen 1 is waiting
                    if self.input_flag2 {
                        self.q2_int.provide_input(&self.input2[0..self.input_offset2]);
                        self.input_offset2 = 0;
                        self.input_flag2 = false;
                    } 
                    let mut output = KernelOutput::new(2, self.screen, self.new_line2);
                    let result: TickResult<()> = self.q2_int.tick(&mut output);
                    self.ticks[self.turn_index - 1] += 1;
                    let tmp = output.return_values();
                    self.screen = tmp.1;
                    self.new_line2 = tmp.2;
                    
                    
                    match result {
                        
                        TickResult::Ok(()) => {
                        },
                        TickResult::Finished => {
                            let s = ['[', 'D', 'O', 'N', 'E', ']'];
                            for c in s {
                                self.edit(c, 2);
                            }
                            //self.draw();
                            self.q2_run = (false, false, true);
                        } ,
                        TickResult::AwaitInput => {
                            self.waiting = true;
                            self.q2_run.1 = true;
                        },
                        TickResult::Err(e) => {
                            println!("{:?}", e);
                            panic!()
                        },
                    }
                    self.draw();
                }     
            }
            if self.q3_run.0 && self.turn_index == 3 {
                if !self.q3_run.1 { //if screen 1 is waiting

                    if self.input_flag3 {
                        //println!("{:?}", self.input);
                        //panic!();
                        self.q3_int.provide_input(&self.input3[0..self.input_offset3]);
                        self.input_offset3 = 0;
                        self.input_flag3 = false;
                    } 
                    let mut output = KernelOutput::new(3, self.screen, self.new_line3);
                    let result: TickResult<()> = self.q3_int.tick(&mut output);
                    self.ticks[self.turn_index - 1] += 1;
                    let tmp = output.return_values();
                    self.screen = tmp.1;
                    self.new_line3 = tmp.2;
                   
                    match result {
                        
                        TickResult::Ok(()) => {
                        },
                        TickResult::Finished => {
                            let s = ['[', 'D', 'O', 'N', 'E', ']'];
                            for c in s {
                                self.edit(c, 3);
                            }
                            //self.draw();
                            self.q3_run = (false, false, true);
                        } ,
                        TickResult::AwaitInput => {
                            self.waiting = true;
                            self.q3_run.1 = true;
                        },
                        TickResult::Err(e) => {
                            println!("{:?}", e);
                            panic!()
                        },
                }
                self.draw();
    
            }     
            }
            if self.q4_run.0 && self.turn_index == 4 {
                    if !self.q4_run.1 { //if screen 1 is waiting
                        if self.input_flag1 {
                            //self.q4_int.provide_input(&self.input[0..self.input_offset]);
                            self.input_offset1 = 0;
                            self.input_flag1 = false;
                        } 
                        let mut output = KernelOutput::new(self.active, self.screen, self.new_line);
                        //let result: TickResult<()> = self.q4_int.tick(&mut output);
                        self.ticks[self.turn_index] += 1;
                        let tmp = output.return_values();
                        self.screen = tmp.1;
                        self.new_line = tmp.2;
                        self.draw();
        
                    //     match result {
                            
                    //         TickResult::Ok(()) => {
                    //         },
                    //         TickResult::Finished => {
                    //             let s = ['[', 'D', 'O', 'N', 'E', ']'];
                    //             for c in s {
                    //                 self.edit(c);
                    //             }
                    //             self.draw();
                    //             self.q4_run = (false, false);
                    //         } ,
                    //         TickResult::AwaitInput => {
                    //             self.waiting = true;
                    //             self.q4_run.1 = true;
                    //         },
                    //         TickResult::Err(e) => {
                    //             println!("{:?}", e);
                    //             panic!()
                    //         },
                    // }
                }   
                self.turn_index += 1;   
            } 
            self.turn_index += 1;
            self.turn_index = self.turn_index % 4;
        }
        
        

    }

    fn handle_unicode(&mut self, key: char) {
        let mut activate = false;
        if !self.editing && self.active != 5 && (!self.q1_run.0 && !self.q2_run.0 && !self.q3_run.0 && !self.q4_run.0) {
            if key == 'e' {
                self.read_file_to_window();
                activate = true;
            } else{
                activate = false;
            }
            
        } 
        
        if !self.q1_run.0 || !self.q2_run.0 || !self.q3_run.0 || !self.q4_run.0 {
            if key == 'r' {
                self.run();
                activate = true;
            }
        
        }

        if !activate {
            if is_drawable(key) && self.waiting && key != '\n' {
                self.edit(key, self.active);
                if self.active == 1{
                    self.input1[self.input_offset1] = key;
                    self.input_offset1 += 1;
                } else if self.active == 2{
                    self.input2[self.input_offset2] = key;
                    self.input_offset2 += 1;
                } else if self.active == 3{
                    self.input3[self.input_offset3] = key;
                    self.input_offset3 += 1;
                }
                

            } else if key.is_alphanumeric() && self.active == 5{
                let start = FILENAME_PROMPT.len();
                let mut count: usize = 0; 
                for i in start..start+MAX_FILENAME_BYTES {
                    if count == MAX_FILENAME_BYTES {
                        break;
                    }
                    if self.screen[0][i] == ' '{
                        self.screen[0][i] = key;
                        break;
                    }
                    count += 1
                }
                
            }else if is_drawable(key) && self.editing {
                self.edit(key, self.active)
            } else if key == '\u{08}'{
                if self.active == 5 {
                    let start = FILENAME_PROMPT.len();
                    for i in start..start+MAX_FILENAME_BYTES {
                        if self.screen[0][i] == ' '{
                            self.screen[0][i - 1] = ' ';
                            break;
                        }
                    }
                } else if self.editing{
                    self.edit(key, self.active)
                }
                
            } else if key == '\n'{
                if self.active == 5 {
                    self.create_file();
                }
                if self.editing {
                    self.edit(key, self.active);
                }
                if self.waiting {      
                    if self.active == 1 {
                        self.new_line1 = true;
                        self.q1_run.1 = false;
                        self.input_flag1 = true;
                    } else if self.active == 2 {
                        self.new_line2 = true;
                        self.q2_run.1 = false;
                        self.input_flag2 = true;
                    } else if self.active == 3 {
                        self.new_line3 = true;
                        self.q3_run.1 = false;
                        self.input_flag3 = true;
                    } else if self.active == 4 {
                        self.q4_run.1 = false;
                    }
                    self.waiting_check();
                    //self.waiting = false;
                }
                
            } 
        }
    }
        
    fn waiting_check(&mut self){
        if !self.q1_run.1 && !self.q2_run.1 && !self.q3_run.1 && self.q4_run.1{   // add the rest 
            self.waiting = false
        }
    }

    fn highlight(&mut self, dir: char){
        let directory = self.files.list_directory().unwrap();
        let file_count = directory.0;
        if !self.editing {
            if dir == 'r' && self.buffer_offset < file_count - 1{
                self.buffer_offset += 1;
                self.move_highlight();
            } else if dir == 'l' && self.buffer_offset != 0{
                self.buffer_offset -= 1;
                self.move_highlight();
            } else if dir == 'u' && self.buffer_offset > 2{
                self.buffer_offset -= 3;
                self.move_highlight();
            }else if dir == 'd' && self.buffer_offset < file_count - 3{
                self.buffer_offset += 3;
                self.move_highlight();
            }
        }
        
    }

    fn move_highlight(&mut self) {
        if self.active == 1{
            let start = 1 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1));
            let buff = self.screen[self.buffer_offset / 3 + 2];
            let mut count = 0;
            for i in start..start + MAX_FILENAME_BYTES{
                self.q1_buffer[count] = buff[i];
                count += 1;
            }
        } else if self.active == 2 {
            let start = WINDOW_WIDTH + 3 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1));
            let buff = self.screen[self.buffer_offset / 3 + 2];
            let mut count = 0;
            for i in start..start + MAX_FILENAME_BYTES{
                self.q2_buffer[count] = buff[i];
                count += 1;
            }
        } else if self.active == 3 {
            let start = 1 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1));
            let buff = self.screen[self.buffer_offset / 3 + 3 + WINDOW_HEIGHT];
            let mut count = 0;
            for i in start..start + MAX_FILENAME_BYTES{
                self.q3_buffer[count] = buff[i];
                count += 1;
            }
        } else if self.active == 4 {
            let start = WINDOW_WIDTH + 3 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1));
            let buff = self.screen[self.buffer_offset / 3 + 3 + WINDOW_HEIGHT];
            let mut count = 0;
            for i in start..start + MAX_FILENAME_BYTES{
                self.q4_buffer[count] = buff[i];
                count += 1;
            }
        }
    }
    
    pub fn draw(&mut self) {
        self.add_files(false);
        for i in 0..BUFFER_HEIGHT{
            for j in 0..BUFFER_WIDTH{
                plot(self.screen[i][j], j, i, ColorCode::new(Color::White, Color::Black));
            }
        }
        
        if self.editing {
            self.setup_editing_window();
        }
        self.draw_highlight();
        
        
    }

    fn draw_highlight(&mut self) {
        for i in 0..MAX_FILENAME_BYTES + 1{
            if self.active == 1{
                if !self.editing && !self.q1_run.2 {
                    plot(self.q1_buffer[i], i + 1 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1)), self.buffer_offset / 3 + 2, ColorCode::new(Color::Black, Color::White));
                }
                if !self.q2_run.2{
                    plot(self.q2_buffer[i], i + 1 + WINDOW_WIDTH + 2, 2, ColorCode::new(Color::Black, Color::White));
                }
                if !self.q3_run.2{
                    plot(self.q3_buffer[i], i + 1, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
                }
                plot(self.q4_buffer[i], i + WINDOW_WIDTH + 3, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
            }else if self.active == 2 {
                if !self.q1_run.2{
                    plot(self.q1_buffer[i], i + 1, 2, ColorCode::new(Color::Black, Color::White));
                } 
                if !self.editing && !self.q2_run.2 {
                    plot(self.q2_buffer[i], i + WINDOW_WIDTH + 3 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1)),self.buffer_offset / 3 + 2, ColorCode::new(Color::Black, Color::White));
                }
                if !self.q3_run.2 {
                    plot(self.q3_buffer[i], i + 1, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
                }
                plot(self.q4_buffer[i], i + WINDOW_WIDTH + 3, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
            } else if self.active == 3 {
                if !self.q1_run.2{
                    plot(self.q1_buffer[i], i + 1, 2, ColorCode::new(Color::Black, Color::White));
                } 
                if !self.q2_run.2 {
                    plot(self.q2_buffer[i], i + 1 + WINDOW_WIDTH + 2, 2, ColorCode::new(Color::Black, Color::White));
                }
                if !self.editing && !self.q3_run.2 {
                    plot(self.q3_buffer[i], i + 1 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1)),self.buffer_offset / 3 + 3 + WINDOW_HEIGHT, ColorCode::new(Color::Black, Color::White));
                }
                plot(self.q4_buffer[i], i + WINDOW_WIDTH + 3, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
            } else if self.active == 4 {
                plot(self.q1_buffer[i], i + 1, 2, ColorCode::new(Color::Black, Color::White));
                plot(self.q2_buffer[i], i + 1 + WINDOW_WIDTH + 2, 2, ColorCode::new(Color::Black, Color::White));
                plot(self.q3_buffer[i], i+ 1, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
                if !self.editing && !self.q4_run.2 {
                    plot(self.q4_buffer[i], i  +WINDOW_WIDTH + 3 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1)), self.buffer_offset / 3 + WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
                }
               
            }
        }
    }

    fn tick_numbers(&mut self, tick_num: usize) -> (char, char, char, char) {
        let mut spot1 = '0';
        let mut spot2 = '0';
        let mut spot3 = '0';
        let mut spot4 = '0';
        let mut count = 4;

        
        
        spot4 = char::from_digit((self.ticks[tick_num] % 10) as u32, 10).unwrap();
        spot3 =  char::from_digit((self.ticks[tick_num] / 10 % 10)as u32, 10).unwrap();
        spot2 = char::from_digit((self.ticks[tick_num]/ 10 / 10 % 10) as u32, 10).unwrap();
        spot1 = char::from_digit((self.ticks[tick_num]/ 10 / 10 / 10 % 10) as u32, 10).unwrap();
        let mut spots = [spot1, spot2, spot3, spot4];
        //
        for s in spots {
            if s != '0' || count == 1{
                break;
            } else {
                count -= 1;
            }
        }
        
        if count == 3 {
            spot1 = spot2;
            spot2 = spot3;
            spot3 = spot4;
            spot4 = ' ';
        } else if count == 2 {
            spot1 = spot3;
            spot2 = spot4;
            spot3 = ' ';
            spot4 = ' ';
        } else if count == 1 {
            spot1 = spot4;
            spot2 = ' ';
            spot3 = ' ';
            spot4 = ' ';
        }
        return (spot1, spot2, spot3, spot4)
    }

    pub fn draw_proc_status(&mut self) {
        self.screen[0][WINDOWS_WIDTH + 1] = 'F';
        self.screen[0][WINDOWS_WIDTH + 2] = '1';
        let ticks1 = self.tick_numbers(0);
        self.screen[1][WINDOWS_WIDTH + 1] = ticks1.0;
        self.screen[1][WINDOWS_WIDTH + 2] = ticks1.1;
        self.screen[1][WINDOWS_WIDTH + 3] = ticks1.2;
        self.screen[1][WINDOWS_WIDTH + 4] = ticks1.3;

        self.screen[2][WINDOWS_WIDTH + 1] = 'F';
        self.screen[2][WINDOWS_WIDTH + 2] = '2';
        let ticks2 = self.tick_numbers(1);
        self.screen[3][WINDOWS_WIDTH + 1] = ticks2.0;
        self.screen[3][WINDOWS_WIDTH + 2] = ticks2.1;
        self.screen[3][WINDOWS_WIDTH + 3] = ticks2.2;
        self.screen[3][WINDOWS_WIDTH + 4] = ticks2.3;

        self.screen[4][WINDOWS_WIDTH + 1] = 'F';
        self.screen[4][WINDOWS_WIDTH + 2] = '3';
        let ticks3 = self.tick_numbers(2);
        self.screen[5][WINDOWS_WIDTH + 1] = ticks3.0;
        self.screen[5][WINDOWS_WIDTH + 2] = ticks3.1;
        self.screen[5][WINDOWS_WIDTH + 3] = ticks3.2;
        self.screen[5][WINDOWS_WIDTH + 4] = ticks3.3;

        self.screen[6][WINDOWS_WIDTH + 1] = 'F';
        self.screen[6][WINDOWS_WIDTH + 2] = '4';
        let ticks4 = self.tick_numbers(3);
        self.screen[7][WINDOWS_WIDTH + 1] = ticks4.0;
        self.screen[7][WINDOWS_WIDTH + 2] = ticks4.1;
        self.screen[7][WINDOWS_WIDTH + 3] = ticks4.2;
        self.screen[7][WINDOWS_WIDTH + 4] = ticks4.3;

        for i in 0..NUM_WINDOWS * 2 {
            for j in WINDOWS_WIDTH.. WINDOWS_WIDTH + TASK_MANAGER_WIDTH {
                plot(self.screen[i][j], j, i, ColorCode::new(Color::White, Color::Black))
            }
        }
    }

    
}

pub struct  KernelOutput {
    active : usize,
    screen : [[char; BUFFER_WIDTH]; BUFFER_HEIGHT],
    new_line: bool,

}

impl KernelOutput{
    fn new(active: usize, screen: [[char; BUFFER_WIDTH]; BUFFER_HEIGHT] , new_line: bool) -> Self{
        let mut active = active;
        let mut screen = screen;
        let mut new_line = new_line;
        Self{active, screen, new_line}
    }

    fn return_values(&mut self) -> (usize, [[char; BUFFER_WIDTH]; BUFFER_HEIGHT], bool){
        (self.active, self.screen, self.new_line)
    }
}

impl InterpreterOutput for KernelOutput{
    fn print(&mut self, chars: &[u8]) {
        let mut last_char = false;
        let mut spot = (0,0);    
        if self.active == 1 {
            for i in FIRST_BORDER_ROW + 1 .. MID_HEIGHT {
                for j in 1..MID_WIDTH{
                    if self.screen[i][j] == ' ' && !last_char {
                        spot = (i, j); 
                        last_char = true;
                    }
                    if self.screen[i][j] != ' ' && last_char {
                        last_char = false;
                    }
                }

            }
        } else if self.active == 2{
            for i in FIRST_BORDER_ROW + 1 .. MID_HEIGHT {
                for j in MID_WIDTH + 1..WINDOWS_WIDTH{
                    if self.screen[i][j] == ' ' && !last_char {
                        spot = (i, j); 
                        last_char = true;
                    }
                    if self.screen[i][j] != ' ' && last_char {
                        last_char = false;
                    }
                }

            }
        } else if self.active == 3{
            for i in MID_HEIGHT + 1 .. LAST_BORDER_ROW {
                for j in 1..MID_WIDTH{
                    if self.screen[i][j] == ' ' && !last_char {
                        spot = (i, j); 
                        last_char = true;
                    }
                    if self.screen[i][j] != ' ' && last_char {
                        last_char = false;
                    }
                }

            }
        } else if self.active == 4{
            for i in MID_HEIGHT + 1 .. LAST_BORDER_ROW {
                for j in MID_WIDTH + 1..WINDOWS_WIDTH{
                    if self.screen[i][j] == ' ' && !last_char {
                        spot = (i, j); 
                        last_char = true;
                    }
                    if self.screen[i][j] != ' ' && last_char {
                        last_char = false;
                    }
                }

            }
        }
        if self.new_line {
            self.new_line = false;
            spot.0 = spot.0 + 1;
            if self.active == 1 || self.active == 3 {
                spot.1 = 1;
            } else {
                //println!("{:?}", spot);
                //panic!();
                spot.1 = MID_WIDTH + 1
            }
           
        }
        
        for char in chars {
            if *char == ('\n' as u8) {
                if spot.0 + 1 == MID_HEIGHT {
                    //Do something with scrolling?
                } else  {
                    self.new_line = true;
                }
            } else{
                self.screen[spot.0][spot.1] = *char as char;
                if spot.1 + 1 == MID_WIDTH {
                    spot = (spot.0 + 1, 1)
                } else {
                    spot = (spot.0, spot.1 + 1)
                }

            }
        }

    }
}


fn text_color() -> ColorCode {
    ColorCode::new(Color::White, Color::Black)
}

fn highlight_color() -> ColorCode {
    ColorCode::new(Color::Black, Color::White)
}

