#[derive(Copy, Clone)]
pub struct Rgbw {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub white: f32,
}

#[derive(Copy, Clone)]
pub struct Rgb {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

pub fn convert_xy_to_rgb(x: f32, y: f32) -> Option<Rgb> {

    // Avoid division by zero and a negative XYZ Z component, passing this check doesn't guarantee
    // the color fits our RGB gamut
    if y == 0.0 || x + y > 1.0 {
        return None;
    }

    // Reconstruct XYZ with an arbitrary reference luminance of 1
    let cie_x = x / y;
    let cie_y = 1.0;
    let cie_z = (1.0 - x - y) / y;

    // XYZ -> linear sRGB, using rounded matrix coefficients
    let red = 3.240970 * cie_x - 1.537383 * cie_y - 0.498611 * cie_z;
    let green = -0.969244 * cie_x + 1.875968 * cie_y + 0.041555 * cie_z;
    let blue = 0.055630 * cie_x - 0.203977 * cie_y + 1.056972 * cie_z;

    // A negative channel means the color lies outside the RGB gamut.
    // Clipping is a simple approximation; it can change the color.
    let red = red.max(0.0);
    let green = green.max(0.0);
    let blue = blue.max(0.0);

    // Preserve channel ratios while setting the strongest channel to 1.
    // Lamp brightness subsequently scales all three channels together.
    let peak = red.max(green).max(blue);
    if peak == 0.0 {
        return None;
    }

    Some(Rgb {
        red: red / peak,
        green: green / peak,
        blue: blue / peak
    })
}

/// Convert given temperature in mireds into RGB approximation
pub fn convert_mireds_to_rgb(mireds: f32) -> Option<Rgb> {
    // This conversion method (for CCT in kelvin to XY) should be quite accurate and was taken from
    // https://colour.readthedocs.io/en/v0.4.1/_modules/colour/temperature/kang2002.html

    // fixme: an ESP32-C6 has no FPU and this may eat too much off our frame budget, or cause issues
    //        with zigbee/RMT. Measure how  much CPU time this costs and decide if we should use a
    //        faster approximation.

    // Reject invalid absolute temperatures before taking the reciprocal
    if !mireds.is_finite() || mireds <= 0.0 {
        return None;
    }

    let kelvin = 1_000_000.0 / mireds;

    // These approximation coefficients are only valid within this range
    if !(1667.0..=25000.0).contains(&kelvin) {
        return None;
    }

    // The formula uses inverse temperature, scaling by 1000 keeps the coefficients readable and
    // avoids large intermediate powers
    let q = 1000.0 / kelvin;
    let q2 = q * q;
    let q3 = q2 * q;

    // Estimate the white point's x chromaticity coordinate
    let x = if kelvin <= 4000.0 {
        -0.2661239 * q3
            - 0.2343589 * q2
            + 0.8776956 * q
            + 0.179910
    } else {
        -3.0258469 * q3
            + 2.1070379 * q2
            + 0.2226347 * q
            + 0.240390
    };

    let x2 = x * x;
    let x3 = x2 * x;

    // Estimate y from x, using the appropriate temperature interval
    let y = if kelvin <= 2222.0 {
        -1.1063814 * x3 - 1.34811020 * x2
            + 2.18555832 * x - 0.20219683
    } else if kelvin <= 4000.0 {
        -0.9549476 * x3 - 1.37418593 * x2
            + 2.09137015 * x - 0.16748867
    } else {
        3.0817580 * x3 - 5.8733867 * x2
            + 3.75112997 * x - 0.37001483
    };

    convert_xy_to_rgb(x, y)
}