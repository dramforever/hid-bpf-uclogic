#![cfg(test)]

#[repr(transparent)]
struct State([u64; 16]);

impl State {
    fn new() -> Self {
        Self([0; 16])
    }
}

const REPORT_SIZE: usize = 12;

struct Vendor([u8; REPORT_SIZE]);

impl Vendor {
    fn parse(hex_str: &str) -> Self {
        let mut hex_str = hex_str.to_owned();
        hex_str.retain(|x| !x.is_ascii_whitespace());
        Vendor(hex::decode(hex_str).unwrap().try_into().unwrap())
    }
}

struct Report(Option<[u8; REPORT_SIZE]>);

fn format_report(report: &Report) -> String {
    match report.0 {
        Some(bytes) => {
            let byte_hex = |b: u8| -> [u8; 3] {
                const HEX: [u8; 16] = *b"0123456789abcdef";
                [b' ', HEX[(b >> 4) as usize], HEX[(b & 0xf) as usize]]
            };
            let hexdump: Vec<u8> = bytes.iter().copied().flat_map(byte_hex).collect();
            let hexdump = str::from_utf8(&hexdump).unwrap();
            let len = bytes.len();
            format!("E: 000000.000000 {len}{hexdump}")
        }
        None => "# No event".to_owned(),
    }
}

fn format_reports(reports: &[Report]) -> String {
    use std::borrow::Cow;

    let add_newline = |s: String| -> [Cow<'static, str>; 2] { [s.into(), "\n".into()] };
    reports
        .iter()
        .map(format_report)
        .flat_map(add_newline)
        .collect()
}

unsafe extern "C" {
    #[link_name = "fixup_report"]
    unsafe fn c_fixup_report(new: *mut u8, old: *const u8, st: *mut State) -> u8;
}

fn fixup_report(report: &Vendor, st: &mut State) -> Report {
    let mut result = [0; REPORT_SIZE];
    let res = unsafe { c_fixup_report(result.as_mut_ptr(), report.0.as_ptr(), st) };
    Report((res != 0).then_some(result))
}

fn run_reports(reports: &[&str]) -> Vec<Report> {
    let mut st = State::new();
    reports
        .iter()
        .map(|&r| fixup_report(&Vendor::parse(r), &mut st))
        .collect()
}

#[test]
fn test_button() {
    let reports = [
        "08 e0 01 01 01 00 00 00 00 00 00 00", // Button 1 press
        "08 e0 01 01 00 00 00 00 00 00 00 00", // Button 1 release
        "08 e0 01 01 00 10 00 00 00 00 00 00", // Button 13 press
        "08 e0 01 01 00 00 00 00 00 00 00 00", // Button 13 release
    ];
    insta::assert_snapshot!(format_reports(&run_reports(&reports)));
}

#[test]
fn test_dial() {
    let reports = [
        "08 f1 01 01 00 01 00 00 00 00 00 00", // Top wheel CW
        "08 f1 01 01 00 02 00 00 00 00 00 00", // Top wheel CCW
        "08 f1 01 02 00 01 00 00 00 00 00 00", // Bottom wheel CW
        "08 f1 01 02 00 02 00 00 00 00 00 00", // Bottom wheel CCW
    ];
    insta::assert_snapshot!(format_reports(&run_reports(&reports)));
}

#[test]
fn test_pen_movement() {
    let reports = [
        "08 80 a0 05 08 0a 00 00 00 00 00 00", // Pen hovering near top left
        "08 00 a0 05 08 0a 00 00 00 00 00 00", // Pen away
        "08 80 e9 c9 04 86 00 00 00 00 00 00", // Pen hovering near bottom right
        "08 00 e9 c9 04 86 00 00 00 00 00 00", // Pen away
    ];
    insta::assert_snapshot!(format_reports(&run_reports(&reports)));
}

#[test]
fn test_pen_tilt() {
    let reports = [
        "08 80 92 16 b6 0c 00 00 00 00 da 00", // Tilt left
        "08 80 0c 0a 5a 0d 00 00 00 00 2e 00", // Tilt right
        "08 80 d1 07 aa 10 00 00 00 00 00 29", // Tilt up
        "08 00 91 0d 63 08 00 00 00 00 00 d9", // Tilt down
    ];
    insta::assert_snapshot!(format_reports(&run_reports(&reports)));
}

#[test]
fn test_pen_buttons() {
    let reports = [
        "08 82 d8 00 77 07 00 00 00 00 00 00", // Press lower button
        "08 84 22 06 26 0c 00 00 00 00 00 00", // Press upper button
    ];
    insta::assert_snapshot!(format_reports(&run_reports(&reports)));
}

#[test]
fn test_pen_pressure() {
    let reports = [
        "08 81 03 00 64 09 21 03 00 00 00 00", // Tap low pressure
        "08 81 e0 03 8b 0d ff 1f 00 00 00 00", // Tap max pressure
    ];
    insta::assert_snapshot!(format_reports(&run_reports(&reports)));
}
