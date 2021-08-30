/*******************************************************************************
 *
 * kit/kernel/serial.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Serial console for debugging

use core::fmt;

#[derive(Debug)]
pub enum Error {
    SerialIsFaulty,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::SerialIsFaulty =>
                write!(f, "serial port is faulty"),
        }
    }
}

pub struct SerialPort {
    io_addr: u16,
}

pub fn com1() -> SerialPort {
    SerialPort { io_addr: 0x3F8 }
}

impl SerialPort {
    pub fn initialize(&mut self) -> Result<(), Error> {
        unsafe {
            // Shamelessly copied from https://wiki.osdev.org/Serial_Ports
            //
            // TODO: break this out into proper constants, understanding of registers
            outb(self.io_addr + 1, 0x00); // Disable all interrupts
            outb(self.io_addr + 3, 0x80); // Enable DLAB (set baud rate divisor)
            outb(self.io_addr + 0, 0x03); // Set divisor to 3 (lo byte) 38400 baud
            outb(self.io_addr + 1, 0x00); //                  (hi byte)
            outb(self.io_addr + 3, 0x03); // 8 bits, no parity, one stop bit
            outb(self.io_addr + 2, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
            outb(self.io_addr + 4, 0x0B); // IRQs enabled, RTS/DSR set
            outb(self.io_addr + 4, 0x1E); // Set in loopback mode, test the serial chip
            // Test serial chip (send byte 0xAE and check if serial returns same byte)
            outb(self.io_addr + 0, 0xAE);

            // Check if serial is faulty (i.e: not same byte as sent)
            if inb(self.io_addr + 0) != 0xAE {
                return Err(Error::SerialIsFaulty);
            }

            // If serial is not faulty set it in normal operation mode
            // (not-loopback with IRQs enabled and OUT#1 and OUT#2 bits enabled)
            outb(self.io_addr + 4, 0x0F);
            Ok(())
        }
    }

    pub fn transmit_ready(&self) -> bool {
        unsafe {
            inb(self.io_addr + 5) & 0x20 != 0
        }
    }

    pub fn write_byte(&mut self, byte: u8) -> Result<(), Error> {
        while !self.transmit_ready() { core::hint::spin_loop() }

        unsafe {
            outb(self.io_addr, byte);
        }
        Ok(())
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        for byte in bytes {
            self.write_byte(*byte)?;
        }
        Ok(())
    }
}

impl fmt::Write for SerialPort {
    fn write_char(&mut self, ch: char) -> fmt::Result {
        let mut buf = [0; 4];
        self.write_bytes(ch.encode_utf8(&mut buf).as_bytes())
            .map_err(|_| fmt::Error)
    }

    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_bytes(s.as_bytes())
            .map_err(|_| fmt::Error)
    }
}

unsafe fn inb(addr: u16) -> u8 {
    let byte: u8;
    asm!("in al, dx", out("al") byte, in("dx") addr);
    byte
}

unsafe fn outb(addr: u16, byte: u8) {
    asm!("out dx, al", in("dx") addr, in("al") byte);
}
