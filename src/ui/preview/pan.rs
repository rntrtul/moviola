use crate::ui::preview::preview::Preview;
use relm4::gtk::graphene;

impl Preview {
    pub(crate) fn pan_preview(&self, offset_x: f32, offset_y: f32) {
        let preview_rect = self.preview_rect();

        // todo: maybe update preview rect on resize and store. stop recomputes on all drag events
        let min_translate_x =
            -(preview_rect.width() - (preview_rect.width() / self.zoom.get() as f32));
        let min_translate_y =
            -(preview_rect.height() - (preview_rect.height() / self.zoom.get() as f32));

        let translate_x = (self.translate.get().x() + offset_x).clamp(min_translate_x, 0f32);
        let translate_y = (self.translate.get().y() + offset_y).clamp(min_translate_y, 0f32);

        self.translate
            .set(graphene::Point::new(translate_x, translate_y));
    }
}
