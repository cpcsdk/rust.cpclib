/// ! Utility code related to OCP

pub fn compress<D: as_slice::AsSlice<Element = u8>>(data: D) -> Vec<u8> {
    eprintln!("[WARNING] OCP compression has never been tested");

    let data = data.as_slice();
    const MARKER: u8 = 1;

    let mut res = Vec::new();

    res.push(b'M');
    res.push(b'J');
    res.push(b'H');

    let length = data.len();
    let high = (length >> 8) as u8;
    let low = (length % 256) as u8;

    res.push(low);
    res.push(high);

    let mut previous = 0;
    let mut count = 0;

    for current in &data[1..] {
        let current = *current;

        if current == MARKER {
            if count != 0 {
                res.push(MARKER);
                res.push(count);
                res.push(previous);
            }

            res.push(MARKER);
            res.push(1);
            res.push(MARKER);
        }
        else if previous == current {
            if count == 255 {
                res.push(MARKER);
                res.push(0);
                res.push(current);
                count = 0;
            }
            else {
                count += 1;
            }
        }
        else {
            if count == 1 {
                debug_assert!(MARKER != current);
                res.push(current);
            }
            else {
                res.push(MARKER);
                res.push(count);
                res.push(current);
            }
            count = 0;
        }

        previous = current;
    }

    if count == 1 {
        res.push(previous);
    }
    else if count > 1 {
        res.push(MARKER);
        res.push(count);
        res.push(previous);
    }

    res
}
