use std::io::prelude::*;
use std::fs::File;

const PC_START: u16 = 0x3000;
const UINT16_MAX: u16 = 65536;
const ALLMEMORY: [u16; UINT16_MAX as usize] = [0; UINT16_MAX];

#[derive(Clone, Copy)]
enum Register {
    RR0 = 0,
    RR1,
    RR2,
    RR3,
    RR4,
    RR5,
    RR6,
    RR7,
    RPC,
    RCOND,
    RCOUNT,
}

// register in array
const REG: [Register; 10] = [
    Register::RR0,
    Register::RR1,
    Register::RR2,
    Register::RR3,
    Register::RR4,
    Register::RR5,
    Register::RR6,
    Register::RR7,
    Register::RPC,
    Register::RCOND,
];


// instructions

enum Instruction {
    BR = 0,
    // branch
    ADD,
    // add
    LD,
    // load
    ST,
    // store
    JSR,
    // jump register
    AND,
    // bitwise and
    LDR,
    // load register
    STR,
    // store register
    RTI,
    // unused
    NOT,
    // bitwise not
    LDI,
    // load indirect
    STI,
    // store indirect
    JMP,
    // jump
    RES,
    // reserved (unused)
    LEA,
    // load effective address
    TRAP,   // execute trap
}

// condition flags
enum ConditionFlag {
    POS = 1 << 0,
    // P
    ZRO = 1 << 1,
    // Z
    NEG = 1 << 2, // N
}

// trap
enum TrapCode {
    GETC = 0x20,
    OUT = 0x21,
    PUTS = 0x22,
    IN = 0x23,
    PUTSP = 0x24,
    HALT = 0x25,
}

enum MemoryMappedRegister {
    KBSR = 0xFE00, // keyboard status
    KBDR = 0xFE02, // keyboard data
}

fn sign_extend(x: u16, bit_count: u16) -> u16 {
    if (x >> (bit_count - 1)) & 1 {
        x | (0xFFFF << bit_count);
    }
    x
}

fn update_flags(r: u16) {
    if REG[r] == 0 {
        REG[Register::RCOND] = ConditionFlag::ZRO;
    } else if REG[r] >> 15 {
        REG[Register::RCOND] = ConditionFlag::NEG;
    } else {
        REG[Register::RCOND] = ConditionFlag::POS;
    }
}

fn read_image_file(file: &mut File) {
    let origin: u16 = file.read_u16::<LittleEndian>().unwrap();
    let max_read: u16 = UINT16_MAX - origin;
    let mut buffer: Vec<u16> = Vec::new();
    let mut read: u16 = origin;
    while read < max_read {
        let val: u16 = file.read_u16::<LittleEndian>().unwrap();
        buffer.push(val);
        read += 1;
    }
    let origin_location: usize = origin as usize;
    let buffer_location: usize = 0;
    let buffer_size: usize = buffer.len();
    ALLMEMORY[origin_location..buffer_size].copy_from_slice(&buffer[buffer_location..buffer_size]);
}

fn read_image(path: &str) {
    let mut file = File::open(path).unwrap();
    read_image_file(&mut file);
}

fn mem_write(address: u16, val: u16) {
    ALLMEMORY[address as usize] = val;
}

fn mem_read(address: u16) -> u16 {
    if address == MemoryMappedRegister::KBSR as u16 {
        if check_key() {
            ALLMEMORY[MemoryMappedRegister::KBSR as usize] = 1 << 15;
            ALLMEMORY[MemoryMappedRegister::KBDR as usize] = std::io::stdin().bytes().next().unwrap().unwrap() as u16;
        } else {
            ALLMEMORY[MemoryMappedRegister::KBSR as usize] = 0;
        }
    }
    ALLMEMORY[address as usize]
}

fn main() {
    REG[Register::RPC] = PC_START;

    let mut running: i32 = 1;
    while running {
        let instr = mem_read(REG[Register::RPC]);
        REG[Register::RPC] += 1;
        let op = instr >> 12;

        match op {
            Instruction::ADD => {
                let dr = (instr >> 9) & 0x7;
                let sr1 = (instr >> 6) & 0x7;
                let imm_flag = (instr >> 5) & 0x1;
                if imm_flag == 1 {
                    let imm5 = sign_extend(instr & 0x1F, 5);
                    REG[dr] = REG[sr1] + imm5;
                } else {
                    let sr2 = instr & 0x7;
                    REG[dr] = REG[sr1] + REG[sr2];
                }
                update_flags(dr);
            }

            Instruction::AND => {
                let dr = (instr >> 9) & 0x7;
                let sr1 = (instr >> 6) & 0x7;
                let imm_flag = (instr >> 5) & 0x1;
                if imm_flag == 1 {
                    let imm5 = sign_extend(instr & 0x1F, 5);
                    REG[dr] = REG[sr1] & imm5;
                } else {
                    let sr2 = instr & 0x7;
                    REG[dr] = REG[sr1] & REG[sr2];
                }
            }
            Instruction::NOT => {
                let dr = (instr >> 9) & 0x7;
                let sr1 = (instr >> 6) & 0x7;
                REG[dr] = !REG[sr1];
                update_flags(dr);
            }
            Instruction::BR => {
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                let cond_flag = (instr >> 9) & 0x7;
                if cond_flag & REG[Register::RCOND] != 0 {
                    REG[Register::RPC] += pc_offset;
                }
            }
            Instruction::JMP => {
                let base_r = (instr >> 6) & 0x7;
                REG[Register::RPC] = REG[base_r];
            }
            Instruction::JSR => {
                REG[Register::RR7] = REG[Register::RPC];
                let flag = (instr >> 11) & 1;
                if flag == 0 {
                    let base_r = (instr >> 6) & 0x7;
                    REG[Register::RPC] = REG[base_r];
                } else {
                    let pc_offset = sign_extend(instr & 0x7FF, 11);
                    REG[Register::RPC] += pc_offset;
                }
            }
            Instruction::LD => {
                let dr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                REG[dr] = mem_read(REG[Register::RPC] + pc_offset);
                update_flags(dr);
            }
            Instruction::LDI => {
                let dr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                REG[dr] = mem_read(mem_read(REG[Register::RPC] + pc_offset));
                update_flags(dr);
            }
            Instruction::LDR => {
                let dr = (instr >> 9) & 0x7;
                let base_r = (instr >> 6) & 0x7;
                let offset = sign_extend(instr & 0x3F, 6);
                REG[dr] = mem_read(REG[base_r] + offset);
                update_flags(dr);
            }
            Instruction::LEA => {
                let dr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                REG[dr] = REG[Register::RPC] + pc_offset;
                update_flags(dr);
            }
            Instruction::ST => {
                let sr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                mem_write(REG[Register::RPC] + pc_offset, REG[sr]);
            }
            Instruction::STI => {
                let sr = (instr >> 9) & 0x7;
                let pc_offset = sign_extend(instr & 0x1FF, 9);
                mem_write(mem_read(REG[Register::RPC] + pc_offset), REG[sr]);
            }
            Instruction::STR => {
                let sr = (instr >> 9) & 0x7;
                let base_r = (instr >> 6) & 0x7;
                let offset = sign_extend(instr & 0x3F, 6);
                mem_write(REG[base_r] + offset, REG[sr]);
            }
            Instruction::TRAP => {
                match instr & 0xFF {
                    Trap::GETC => {
                        REG[Register::RR0] = read_char();
                    }
                    Trap::OUT => {
                        put_char(REG[Register::RR0] as u8);
                    }
                    Trap::PUTS => {
                        let mut c = mem_read(REG[Register::RR0]);
                        while c != 0 {
                            put_char(c as u8);
                            REG[Register::RR0] += 1;
                            c = mem_read(REG[Register::RR0]);
                        }
                    }
                    Trap::IN => {
                        print!("Enter a character: ");
                        let c = read_char();
                        put_char(c);
                        REG[Register::RR0] = c as u16;
                    }
                    Trap::PUTSP => {
                        let mut c = mem_read(REG[Register::RR0]);
                        while c != 0 {
                            let c1 = (c & 0xFF) as u8;
                            put_char(c1);
                            let c2 = (c >> 8) as u8;
                            if c2 != 0 {
                                put_char(c2);
                            }
                            REG[Register::RR0] += 1;
                            c = mem_read(REG[Register::RR0]);
                        }
                    }
                    Trap::HALT => {
                        println!("HALT");
                        running = 0;
                    }
                    _ => {
                        println!("Unknown trap code");
                        running = 0;
                    }
                }
            }
            Instruction::RES => {
                println!("RES");
                running = 0;
            }
            Instruction::RTI => {
                println!("RTI");
                running = 0;
            }
            Instruction::BAD => {
                println!("BAD");
                running = 0;
            }
        }
    }
}
