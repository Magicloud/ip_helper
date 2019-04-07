extern crate rayon;
extern crate regex;

use crate::Relation::{Beside, Join, Overlap, Single};
use rayon::prelude::*;
use regex::Regex;
use std::env::args;
use std::fmt;
use std::fs;

#[derive(Clone, Debug)]
struct IPRange {
    start: u32,
    length: u32,
}
impl fmt::Display for IPRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}/{}",
            [3, 2, 1, 0]
                .iter()
                .map(|i| ((self.start >> (i * 8)) as u8).to_string())
                .collect::<Vec<_>>()
                .join("."),
            bit_count(std::u32::MAX - self.length + 1)
        )
    }
}

#[derive(Clone, Debug)]
enum Relation<T> {
    Overlap(T, T),
    Join(T),
    Beside(T, T),
    Single(T),
}

fn bit_count(y: u32) -> u8 {
    let x = y;
    let x = x - ((x >> 1) & 0x55555555);
    let x = (x & 0x33333333) + ((x >> 2) & 0x33333333);
    let x = (x + (x >> 4)) & 0x0F0F0F0F;
    let x = x + (x >> 8);
    let x = x + (x >> 16);
    (x & 0x0000003F) as u8
}

// ipr: 0.0.0.0/8
// ipr: 0.0.0.0/255.0.0.0
fn parse_iprange(ipr: &str) -> Option<IPRange> {
    let parser = Regex::new(
        r"^(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})/(\d+|\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})$",
    )
    .unwrap();
    let caps = parser.captures(ipr)?;
    let ipaddr: u32 = caps
        .get(1)?
        .as_str()
        .split('.')
        .map(|x| x.parse::<u8>().unwrap())
        .enumerate()
        .map(|(i, ips)| (ips as u32) << (8 * (3 - i)))
        .sum();
    let mask_ = caps.get(2).unwrap().as_str();
    let mask: u32 = if mask_.contains(".") {
        mask_
            .split('.')
            .map(|ms| ms.parse::<u8>().unwrap())
            .enumerate()
            .map(|(i, ips)| (ips as u32) << (8 * (3 - i)))
            .sum()
    } else {
        std::u32::MAX << (32 - mask_.parse::<u8>().unwrap())
    };
    Some(IPRange {
        start: ipaddr & mask,
        length: std::u32::MAX - mask + 1,
    })
}

fn check_two_subnets(a: IPRange, b: IPRange) -> Relation<IPRange> {
    if a.start + a.length > b.start {
        Overlap(a, b)
    } else if a.length == b.length
        && a.start + a.length == b.start
        && (a.start & ((std::u32::MAX - a.length + 1) << 1)) == a.start
    {
        Join(IPRange {
            start: a.start,
            length: a.length * 2,
        })
    } else if a.start + a.length == b.start {
        Beside(a, b)
    } else {
        Single(a)
    }
}

fn main() -> Result<(), std::io::Error> {
    let mut v: Vec<_> = fs::read_to_string(args().skip(1).next().unwrap())?
        .lines()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|arg| parse_iprange(&arg).unwrap())
        .collect();
    v.par_sort_by(|a, b| a.start.cmp(&b.start));
    let mut r: Vec<_> = v.into_par_iter().map(|a| Join(a)).collect();
    while r
        .clone()
        .into_par_iter()
        .any(|x| if let Join(_) = x { true } else { false })
    {
        let v = r
            .into_par_iter()
            .filter(|a| if let Overlap(_, _) = a { false } else { true })
            .map(|a| match a {
                Overlap(x, _) => x,
                Beside(x, _) => x,
                Join(x) => x,
                Single(x) => x,
            });
        let mut z: Vec<_> = v.clone().collect();
        z.drain(0..1);
        z.push(IPRange {
            start: 0,
            length: 0,
        });
        r = v
            .collect::<Vec<_>>()
            .into_par_iter()
            .zip(z)
            .map(|(a, b)| check_two_subnets(a, b))
            .collect();
    }
    for k in r {
        match k {
            Overlap(_, _) => (),
            Beside(x, _) => println!("{}", x),
            Join(x) => println!("{}", x),
            Single(x) => println!("{}", x),
        }
    }
    Ok(())
}
