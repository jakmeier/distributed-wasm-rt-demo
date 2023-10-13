use paddle::quicksilver_compat::{Circle, Shape};

use paddle::{
    self, ComplexShape, DisplayArea, FitStrategy, FloatingText, PointerEventType, Rectangle,
    Transform,
};

use paddle::quicksilver_compat::Color;

pub(crate) struct Slider<T> {
    pub(crate) initialized: bool,
    pub(crate) selected: bool,
    pub(crate) values: Vec<T>,
    pub(crate) current: usize,
    pub(crate) title_field: FloatingText,
    pub(crate) text_field: FloatingText,
    pub(crate) area: Rectangle,
    pub(crate) main_area: Rectangle,
    pub(crate) slider_line: Rectangle,
    pub(crate) background: ComplexShape,
    pub(crate) main_color: Color,
    pub(crate) secondary_color: Color,
    pub(crate) knob_color: Color,
    pub(crate) z: i16,
}

impl<T: ToString> Slider<T> {
    pub(crate) fn new(
        area: Rectangle,
        title: String,
        values: Vec<T>,
        main_color: Color,
        secondary_color: Color,
        knob_color: Color,
    ) -> Self {
        let radius = area.height().min(area.width()) / 4.0;
        let background = ComplexShape::rounded_rectangle(area, radius);
        let (header, main) = area
            .padded(radius / 2.0)
            .cut_horizontal(area.height() / 4.0);
        let (mut title_area, value_area) = header.cut_vertical(area.width() / 2.0);
        title_area.pos.x += radius / 2.0;
        let mut title_field = FloatingText::new(&title_area, title).unwrap();
        title_field
            .update_fit_strategy(FitStrategy::LeftCenter)
            .unwrap();
        let mut text_field = FloatingText::new(
            &value_area,
            values.get(0).map(ToString::to_string).unwrap_or_default(),
        )
        .unwrap();
        text_field.update_fit_strategy(FitStrategy::Center).unwrap();
        let mut slider_line = main.padded(radius * 1.5);
        slider_line.size.y /= 10.0;
        slider_line.pos.y += 4.5 * slider_line.size.y;
        Self {
            initialized: false,
            selected: false,
            values,
            current: 0,
            title_field,
            text_field,
            area,
            main_area: main,
            slider_line,
            background,
            main_color,
            secondary_color,
            knob_color,
            z: 0,
        }
    }

    pub(crate) fn set_value_index(&mut self, i: usize) {
        assert!(i < self.values.len());
        self.current = i;
        self.text_field.update_text(&self.value().to_string());
        self.text_field.draw();
    }

    /// Returns true if the value was adjusted.
    pub(crate) fn adjust(&mut self, event: paddle::PointerEvent) -> bool {
        if self.main_area.contains(event.pos()) {
            match event.event_type() {
                PointerEventType::Down => self.selected = true,
                PointerEventType::Up | PointerEventType::Leave => self.selected = false,
                _ => (),
            }

            if self.selected {
                // find closest step
                let discrete_distance = (self.values.len() - 1) as f32;
                let ratio = (event.pos().x - self.slider_line.x()) / self.slider_line.width();
                let float_index = ratio * discrete_distance;
                let index = float_index.round().max(0.0).min(discrete_distance) as usize;
                self.set_value_index(index);
                return true;
            }
        } else {
            self.selected = false;
        }
        false
    }
}

impl<T> Slider<T> {
    pub(crate) fn value(&self) -> &T {
        &self.values[self.current]
    }

    pub(crate) fn draw(&mut self, canvas: &mut DisplayArea) {
        if !self.initialized {
            canvas.add_text(&mut self.text_field);
            canvas.add_text(&mut self.title_field);
        }
        let radius = self.area.height().min(self.area.width()) / 4.0;
        canvas.draw_ex(
            &self.background,
            &self.main_color,
            Transform::default(),
            self.z,
        );

        canvas.draw(&self.slider_line, &self.secondary_color);

        let ratio = self.current as f32 / (self.values.len() - 1) as f32;
        let x = self.slider_line.x() + self.slider_line.width() * ratio;
        let y = self.slider_line.center().y;
        let knob = Circle::new((x, y), radius);
        canvas.draw_ex(&knob, &self.knob_color, Transform::default(), self.z);
    }

    pub(crate) fn active(&self) {
        self.text_field.show().unwrap();
        self.title_field.show().unwrap();
    }

    pub(crate) fn inactive(&self) {
        self.text_field.hide().unwrap();
        self.title_field.hide().unwrap();
    }
}

impl<T: PartialEq + ToString> Slider<T> {
    pub(crate) fn set_value(&mut self, value: &T) {
        if let Some(i) = self.values.iter().position(|v| v == value) {
            self.set_value_index(i);
        }
    }
}
