#![no_std]
#![feature(prelude_2024)]

use filesystem::FileSystem;
// use file_system_solution::{FileSystem, FileSystemResult};
use pc_keyboard::{DecodedKey, KeyCode};
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
    for (filename, contents) in [
        ("hello", HELLO),
        ("nums", NUMS),
        ("add_one", ADD_ONE),
        ("countdown", COUNTDOWN),
        ("average", AVERAGE),
        ("pi", PI),
    ] {
        disk.list_directory();
        let fd = disk.open_create(filename).unwrap();
        disk.write(fd, contents.as_bytes()).unwrap();
        disk.close(fd);
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
        initial_files(&mut files);  
        let process_info= [['+'; TASK_MANAGER_WIDTH]; BUFFER_HEIGHT];
        let file_entry= ['-'; BUFFER_WIDTH];
        let mut active = 1;
        screen = split_screen(screen);
        Self{screen, process_info, file_entry, active, files}
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
            self.screen = update_screen(self.screen, num)
        }
        
        
    }
    pub fn get_filenames(&mut self) {
        let directory = self.files.list_directory();

    }

    fn handle_raw(&mut self, key: KeyCode) {
        match key{
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

            _ => ()
        }
    }

    fn handle_unicode(&mut self, key: char) {
        todo!("handle printable keys");
    }

    pub fn draw(&mut self) {
        //print!(self.screen);
        for i in 0..BUFFER_HEIGHT{
            for j in 0..BUFFER_WIDTH{
                plot(self.screen[i][j], j, i, ColorCode::new(Color::White, Color::Black));
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

