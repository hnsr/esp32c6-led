#[derive(Copy, Clone)]
pub struct Rgbw {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub white: f32,
}

pub fn convert_xy_to_rgbw(x: f32, y: f32) -> Option<Rgbw> {

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

    Some(Rgbw {
        red: red / peak,
        green: green / peak,
        blue: blue / peak,
        white: 0.0,
    })
}

pub fn convert_mireds_to_rgbw(_mireds: f32) -> Rgbw {
    // todo: emulate color temperature using rgb values
    Rgbw {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        white: 1.0,
    }
}