/// Copied from `egui::widgets::Image`
use eframe::egui::*;

#[derive(Clone, Copy, Debug)]
pub struct CustomImage {
    texture_id: TextureId,
    uv: Rect,
    size: Vec2,
    tint: Color32,
    sense: Sense,
    //translate: Vec2,
}

impl CustomImage {
    pub fn new(texture_id: impl Into<TextureId>, size: impl Into<Vec2>) -> Self {
        Self {
            texture_id: texture_id.into(),
            uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            size: size.into(),
            tint: Color32::WHITE,
            sense: Sense::hover(),
            //translate: Vec2::ZERO,
        }
    }

    /*pub fn translate(mut self, translate: Vec2) -> Self {
        self.translate = translate;
        self
    }*/
}

impl CustomImage {
    pub fn paint_at(&self, ui: &mut Ui, rect: Rect) {
        if ui.is_rect_visible(rect) {
            use epaint::*;
            let Self {
                texture_id,
                uv,
                tint,
                ..
            } = self;

            {
                // TODO: builder pattern for Mesh
                let mut mesh = Mesh::with_texture(*texture_id);
                mesh.add_rect_with_uv(rect, *uv, *tint);

                let shape = Shape::mesh(mesh);
                //shape.translate(*translate);

                ui.painter().add(shape);
            }
        }
    }
}

impl Widget for CustomImage {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.size, self.sense);
        self.paint_at(ui, rect);
        response
    }
}
