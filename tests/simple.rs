#![feature(proc_macro_hygiene)]

use numeric_lut::lut;

#[test]
fn normal_access() {
    let lut = lut!(|x @ 0..8, y @ 0..16| -> u32 { x as u32 + y as u32 });
    let x = lut(3, 10);
    assert_eq!(13, x);
}

#[test]
fn normal_access_all() {
    let lut = lut!(|x @ 0..8, y @ 0..16| -> u32 { x as u32 + y as u32 });

    for x in 0..8 {
        for y in 0..16 {
            let r = lut(x, y);
            assert_eq!((x + y) as u32, r);
        }
    }
}

#[test]
fn normal_access_inclusive() {
    let lut = lut!(|x @ 0..=8, y @ 0..=16| -> u32 { x as u32 + y as u32 });
    let x = lut(8, 16);
    assert_eq!(24, x);
}

#[test]
fn normal_access_inclusive_all() {
    let lut = lut!(|x @ 0..=8, y @ 0..=16| -> u32 { x as u32 + y as u32 });

    for x in 0..=8 {
        for y in 0..=16 {
            let r = lut(x, y);
            assert_eq!((x + y) as u32, r);
        }
    }
}

#[test]
#[should_panic]
fn out_of_bounds() {
    let lut = lut!(|x @ 0..8, y @ 0..16| -> u32 { x as u32 + y as u32 });
    lut(10, 3);
}

/*
#[test]
#[should_panic]
fn bad_range() {
    lut!(|x @ 8..0, y @ 8..0| -> u32 { x as u32 + y as u32 });
}
*/
