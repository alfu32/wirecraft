use gpui::*;
pub fn register(cx: &App) -> anyhow::Result<()> {
    cx.text_system().add_fonts(vec![
        std::borrow::Cow::Borrowed(include_bytes!(
            "../../../assets/fonts/MonaspaceNeon-Regular.otf"
        )),
        std::borrow::Cow::Borrowed(include_bytes!(
            "../../../assets/fonts/MonaspaceNeon-Bold.otf"
        )),
        std::borrow::Cow::Borrowed(include_bytes!(
            "../../../assets/fonts/MonaspaceNeon-Italic.otf"
        )),
        std::borrow::Cow::Borrowed(include_bytes!(
            "../../../assets/fonts/MonaspaceNeon-Light.otf"
        )),
        std::borrow::Cow::Borrowed(include_bytes!(
            "../../../assets/fonts/MonaspaceKrypton-Regular.otf"
        )),
    ])?;
    Ok(())
}
