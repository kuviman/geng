use super::*;

pub struct Text<T: AsRef<str>, F: AsRef<Font>> {
    core: WidgetCore,
    text: T,
    font: F,
    size: f32,
    color: Color<f32>,
}

impl<T: AsRef<str>, F: AsRef<Font>> Text<T, F> {
    pub fn new(text: T, font: F, size: f32, color: Color<f32>) -> Self {
        Self {
            core: WidgetCore::void(),
            text,
            font,
            size,
            color,
        }
    }
}

impl<T: AsRef<str>, F: AsRef<Font>> Widget for Text<T, F> {
    fn core(&self) -> &WidgetCore {
        &self.core
    }
    fn core_mut(&mut self) -> &mut WidgetCore {
        &mut self.core
    }
    fn calc_constraints(&mut self) {
        self.core_mut().constraints = widget::Constraints {
            min_size: vec2(
                self.font
                    .as_ref()
                    .measure(self.text.as_ref(), self.size)
                    .width() as f64,
                self.size as f64,
            ),
            flex: vec2(0.0, 0.0),
        };
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        if self.text.as_ref().is_empty() {
            return;
        }
        let size = partial_min(
            self.core().position.height() as f32,
            self.size * self.core().position.width() as f32
                / self
                    .font
                    .as_ref()
                    .measure(self.text.as_ref(), self.size)
                    .width(),
        );
        self.font.as_ref().draw(
            framebuffer,
            self.text.as_ref(),
            self.core().position.bottom_left().map(|x| x as f32),
            size,
            self.color,
        );
    }
}
