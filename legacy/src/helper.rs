use crate::app::Focus;

pub struct KeyBindings {
    pub focus: Focus,
    pub commands: Vec<(String, String)>,
}

impl KeyBindings {
    pub fn new(focus: Focus) -> Self {
        let common_commands = vec![
            ("Tab".to_string(), "Cambiar foco".to_string()),
            ("q".to_string(), "Salir".to_string()),
            ("h".to_string(), "Ayuda".to_string()),
            ("Ctrl + ←/→".to_string(), "Ajustar tamaño".to_string()),
        ];

        let specific_commands = match focus {
            Focus::Sections => vec![
                ("a".to_string(), "Agregar sección".to_string()),
                ("d".to_string(), "Eliminar sección".to_string()),
                ("e".to_string(), "Editar sección".to_string()),
            ],
            Focus::Details => vec![
                ("a".to_string(), "Agregar detalle".to_string()),
                ("d".to_string(), "Eliminar detalle".to_string()),
                ("e".to_string(), "Editar detalle".to_string()),
            ],
            Focus::Search => vec![
                ("Enter".to_string(), "Buscar".to_string()),
                ("Esc".to_string(), "Cancelar búsqueda".to_string()),
            ],
        };

        let mut commands = common_commands;
        commands.extend(specific_commands);

        Self { focus, commands }
    }
}

pub fn get_resize_amount(focus: &Focus) -> (u16, u16) {
    match focus {
        Focus::Sections => (5, 0), // Ajusta el ancho del panel izquierdo
        Focus::Details => (0, 5),  // Ajusta el ancho del panel derecho
        Focus::Search => (0, 0),   // No ajusta nada en modo búsqueda
    }
}
