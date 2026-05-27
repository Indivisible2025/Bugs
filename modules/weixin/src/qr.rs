//! 微信绑定二维码生成

use qrcode::QrCode;
use qrcode::render::unicode;

/// 在终端打印微信绑定二维码
pub fn print_bind_qrcode(port: u16) {
    let bind_url = format!("http://localhost:{}/wechat/bind", port);
    let code = QrCode::new(&bind_url).unwrap();

    let image = code.render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Dark)
        .light_color(unicode::Dense1x2::Light)
        .build();

    // 打印二维码边框
    println!("  ┌{}┐", "─".repeat(image.lines().next().map(|l| l.len()).unwrap_or(40) + 2));
    for line in image.lines() {
        println!("  │ {} │", line);
    }
    println!("  └{}┘", "─".repeat(image.lines().next().map(|l| l.len()).unwrap_or(40) + 2));
}
