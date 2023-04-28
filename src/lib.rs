#![no_std]
#![feature(prelude_2024)]

use filesystem::FileSystem;
// use file_system_solution::{FileSystem, FileSystemResult};
use pc_keyboard::{DecodedKey, KeyCode};
use pluggable_interrupt_os::{println, print};
use pluggable_interrupt_os::vga_buffer::{BUFFER_WIDTH, BUFFER_HEIGHT, plot, ColorCode, Color, plot_str, is_drawable, plot_num};
use ramdisk::RamDisk;
use simple_interp::{Interpreter, InterpreterOutput, i64_into_buffer};
// use gc_heap::CopyingHeap;

// Get rid of some spurious VSCode errors
use core::option::Option;
use core::option::Option::None;
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
const MAX_FILE_BLOCKS: usize = 64;
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
    screen[FIRST_BORDER_ROW][WINDOW_WIDTH + (WINDOW_WIDTH / 2)] = 'F';
    screen[FIRST_BORDER_ROW][WINDOW_WIDTH + (WINDOW_WIDTH / 2) + 1] = '2';
    //F3 header
    screen[MID_HEIGHT][WINDOW_WIDTH / 2] = 'F';
    screen[MID_HEIGHT][WINDOW_WIDTH / 2 + 1] = '3';
    //F4 header
    screen[MID_HEIGHT][WINDOW_WIDTH + (WINDOW_WIDTH / 2)] = 'F';
    screen[MID_HEIGHT][WINDOW_WIDTH + (WINDOW_WIDTH / 2) + 1] = '4';

    screen = update_screen(screen, 1);

    return screen
    

}

pub fn update_screen(mut screen: [[char; BUFFER_WIDTH]; BUFFER_HEIGHT], num: usize) -> [[char; BUFFER_WIDTH]; BUFFER_HEIGHT] {
    for i in 0..BUFFER_HEIGHT {
        for j in 0..BUFFER_WIDTH{
            if (i == FIRST_BORDER_ROW || i == MID_HEIGHT || i == LAST_BORDER_ROW) && j <= WINDOWS_WIDTH && !screen[i][j].is_alphanumeric() { //top,middle,bottom row
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
        let process_info= [['+'; TASK_MANAGER_WIDTH]; BUFFER_HEIGHT];
        let mut active = 1;
        let mut file_count = 0;
        initial_files(&mut files);
        screen = split_screen(screen);
        let file_entry = screen[0];
        let mut def_buffer = [' '; MAX_FILENAME_BYTES + 1];
        let mut q1_buffer = [' '; MAX_FILENAME_BYTES + 1];
        let mut q2_buffer = [' '; MAX_FILENAME_BYTES + 1];
        let mut q3_buffer = [' '; MAX_FILENAME_BYTES + 1];
        let mut q4_buffer = [' '; MAX_FILENAME_BYTES + 1];
        let mut buffer_offset = 0;
        let mut editing = false;
        Self{screen, process_info, file_entry, active, files, file_count, q1_buffer ,q2_buffer,q3_buffer,q4_buffer, buffer_offset,def_buffer, editing }
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
        if self.active != num {
            self.active = num;
            self.reset_buffers();
            self.buffer_offset = 0;
            self.screen = update_screen(self.screen, num);
            
        }
        //self.add_files();
        
        
    }
    fn reset_buffers(&mut self) {
        self.q1_buffer = self.def_buffer;
        self.q2_buffer = self.def_buffer;
        self.q3_buffer = self.def_buffer;
        self.q4_buffer = self.def_buffer;
    }
    
    fn add_files(&mut self) {
        let directory = self.files.list_directory().unwrap();
        let file_count = directory.0;
        let filenames = directory.1;

        if file_count != self.file_count {
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

    fn empty_screen(&mut self) {
        if self.active == 1 {
            for i in FIRST_BORDER_ROW + 1..MID_HEIGHT {
                for j in 1..MID_WIDTH {
                    self.screen[i][j] = ' ';
                }
            }
        }
    }
    fn editing_file(&mut self) {
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
            
        }
    }

    fn handle_unicode(&mut self, key: char) {
        if key == 'e' {
            if self.active == 1{
                let mut buffer = [0; MAX_FILENAME_BYTES];
                for (i,c) in self.q1_buffer.iter().enumerate() {
                    if i == 10 {
                        break;
                    }
                    buffer[i] = *c as u8;
                    self.screen[20][1 + i] = *c;
                }
                let filename = from_utf8(&buffer).unwrap();
                let fd = self.files.open_read(filename).unwrap();
                let mut count = 0;
                let mut file = ['\0' ; 10000];
                let mut buffer = [0;10];
                println!("{:?}", self.files.get_open());
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
                self.editing_file();
                let mut offset = 0;
                for (i, c) in file.iter().enumerate() {
                    if i == count {
                        break;
                    }
                    if i == WINDOW_WIDTH {
                        offset += 1;
                    }
                    self.screen[2 + offset][(i + 1) % WINDOW_WIDTH] = *c;
                }
            }
                

        }
        if key.is_alphanumeric() && self.active == 5{
            let start = FILENAME_PROMPT.len();
            let mut count = 0; 
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
        } else if key == '\u{08}'{
            let start = FILENAME_PROMPT.len();
            for i in start..start+MAX_FILENAME_BYTES {
                if self.screen[0][i] == ' '{
                    self.screen[0][i - 1] = ' ';
                    break;
                }
            }
        } else if key == '\n'{
            //self.screen[20][20] = 'X';
            if self.active == 5 {
                self.create_file();
            }
            
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
        self.add_files();
        for i in 0..BUFFER_HEIGHT{
            for j in 0..BUFFER_WIDTH{
                plot(self.screen[i][j], j, i, ColorCode::new(Color::White, Color::Black));
            }
        }
        self.draw_highlight();
        if self.editing {
            self.editing_file();
        }
        
        
    }

    fn draw_highlight(&mut self) {
        for i in 0..MAX_FILENAME_BYTES + 1{
            if self.active == 1 && !self.editing {
                plot(self.q1_buffer[i], i + 1 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1)), self.buffer_offset / 3 + 2, ColorCode::new(Color::Black, Color::White));
                plot(self.q2_buffer[i], i + 1 + WINDOW_WIDTH + 2, 2, ColorCode::new(Color::Black, Color::White));
                plot(self.q3_buffer[i], i + 1, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
                plot(self.q4_buffer[i], i + WINDOW_WIDTH + 3, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
            }else if self.active == 2 {
                plot(self.q1_buffer[i], i + 1, 2, ColorCode::new(Color::Black, Color::White));
                plot(self.q2_buffer[i], i + WINDOW_WIDTH + 3 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1)),self.buffer_offset / 3 + 2, ColorCode::new(Color::Black, Color::White));
                plot(self.q3_buffer[i], i+ 1, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
                plot(self.q4_buffer[i], i + WINDOW_WIDTH + 3, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
            } else if self.active == 3 {
                plot(self.q1_buffer[i], i + 1, 2, ColorCode::new(Color::Black, Color::White));
                plot(self.q2_buffer[i], i + 1 + WINDOW_WIDTH + 2, 2, ColorCode::new(Color::Black, Color::White));
                plot(self.q3_buffer[i], i + 1 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1)),self.buffer_offset / 3 + 3 + WINDOW_HEIGHT, ColorCode::new(Color::Black, Color::White));
                plot(self.q4_buffer[i], i + WINDOW_WIDTH + 3, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
            } else if self.active == 4 {
                plot(self.q1_buffer[i], i + 1, 2, ColorCode::new(Color::Black, Color::White));
                plot(self.q2_buffer[i], i + 1 + WINDOW_WIDTH + 2, 2, ColorCode::new(Color::Black, Color::White));
                plot(self.q3_buffer[i], i+ 1, WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
                plot(self.q4_buffer[i], i  +WINDOW_WIDTH + 3 + (self.buffer_offset % 3 * (MAX_FILENAME_BYTES + 1)), self.buffer_offset / 3 + WINDOW_HEIGHT + 3, ColorCode::new(Color::Black, Color::White));
            }
        }
    }



    pub fn draw_proc_status(&mut self) {
        //todo!("Draw processor status");
    }

    pub fn run_one_instruction(&mut self) {
        todo!("Run an instruction in a process");
    }
}

fn text_color() -> ColorCode {
    ColorCode::new(Color::White, Color::Black)
}

fn highlight_color() -> ColorCode {
    ColorCode::new(Color::Black, Color::White)
}

