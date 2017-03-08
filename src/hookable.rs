use moduleinfo::ModuleInfo;
use utils;

pub trait HookableFilter {
    fn should_hook(&self, module_info: &ModuleInfo) -> bool;
    fn pick_best_hook_target<'a>(&self, modules: &'a Vec<ModuleInfo>) -> Option<&'a ModuleInfo>;
}

pub trait Hookable : HookableFilter {
    fn module_info(&self) -> Option<&ModuleInfo>;
    fn hook(&mut self, module_info: &ModuleInfo);
    fn unhook(&mut self);
}

pub trait HookableOrderedNameFilter : HookableFilter {
    fn get_current_name_index(&self) -> Option<usize>;
    fn get_names(&self) -> &[&'static str];

    fn compute_current_name_index(&self, module_info: &ModuleInfo) -> Option<usize> {
        self.get_names()
            .iter()
            .position(|&name| {
                let path = utils::get_module_path(module_info.handle).unwrap();
                let filename = path.file_name().unwrap().to_str().unwrap();
                name == filename
            })
    }
}

impl<T> HookableFilter for T where T: HookableOrderedNameFilter {
    fn should_hook(&self, module_info: &ModuleInfo) -> bool {
        let current_index = self.get_current_name_index();

        if current_index == Some(0) {
            // We already have the hook with the top priority.
            return false;
        }

        let path = utils::get_module_path(module_info.handle);

        if path.is_none() {
            return false;
        }

        let path = path.unwrap();

        if path.file_name().is_none() {
            return false;
        }

        let filename = path.file_name().unwrap().to_str().unwrap();

        let names_to_check = match current_index {
            Some(index) => self.get_names().iter().cloned().take(index)
                .collect::<Vec<&'static str>>(),
            None => self.get_names().iter().cloned().collect::<Vec<&'static str>>()
        };

        names_to_check.iter().find(|&name| name == &filename).is_some()
    }

    fn pick_best_hook_target<'a>(&self, modules: &'a Vec<ModuleInfo>) -> Option<&'a ModuleInfo> {
        let modules = modules.iter().filter_map(|module| {
            match utils::get_module_path(module.handle) {
                Some(path) => {
                    match path.file_name() {
                        Some(filename) => Some((module, filename.to_str().unwrap().to_owned())),
                        None => None
                    }
                },
                None => None
            }
        }).collect::<Vec<(&ModuleInfo, String)>>();

        for name in self.get_names() {
            if let Some(module) = modules.iter()
                                         .filter(|&&(_, ref filename)| name == filename)
                                         .next() {
                return Some(module.0);
            }
        }

        None
    }
}
