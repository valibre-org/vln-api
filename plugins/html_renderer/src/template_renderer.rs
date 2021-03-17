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
        let templates: Vec<String> = registry
            .get_templates()
            .into_iter()
            .map(|(id, _template)| id.clone())
            .collect();
        templates
    }

    pub fn register_template(&self, template: &str) -> Result<String, ()> {
        let template_id = {
            use std::hash::{Hash, Hasher};
            let mut hasher = twox_hash::XxHash64::default();
            template.hash(&mut hasher);
            hasher.finish().to_string()
        };

        self.registry
            .borrow_mut()
            .register_template_string(&template_id, template)
            // TODO: map relevant errors to custom enum type.
            .map_err(|_| ())?;

        Ok(template_id)
    }

    pub fn render_template<T>(&self, template_id: &str, data: &T) -> Option<String>
    where
        T: Serialize,
    {
        if self.registry.borrow().has_template(&template_id) {
            return self.registry.borrow().render(&template_id, data).ok();
        }

        None
    }
}
