use handlebars::*;
use serde::Serialize;
use std::{cell::RefCell, rc::Rc};

pub struct TemplateRenderer<'a> {
    registry: Rc<RefCell<Handlebars<'a>>>,
}

impl<'a> Default for TemplateRenderer<'a> {
    fn default() -> Self {
        TemplateRenderer {
            registry: Rc::new(RefCell::new(Handlebars::new())),
        }
    }
}

impl<'a> TemplateRenderer<'a> {
    pub fn get_templates(&self) -> Vec<String> {
        let registry = self.registry.borrow();
        registry
            .get_templates()
            .into_iter()
            .map(|(id, _template)| id.clone())
            .collect()
    }

    pub fn register_template(&self, template: &str) -> Option<String> {
        let template_id = {
            use std::hash::{Hash, Hasher};
            let mut hasher = twox_hash::XxHash64::default();
            template.hash(&mut hasher);
            hasher.finish().to_string()
        };

        self.registry
            .borrow_mut()
            .register_template_string(&template_id, template)
            .ok()?;

        Some(template_id)
    }

    pub fn render_template<T>(&self, template_id: &str, data: &T) -> Option<String>
    where
        T: Serialize,
    {
        self.registry.borrow().render(&template_id, data).ok()
    }
}
