// @spec REQ-CORE-005
// @spec REQ-CORE-006
use crate::error::{SizeOrigin, ZodIssueCode};
use alloc::format;
use alloc::string::String;
use core::sync::atomic::{AtomicU8, Ordering};

/// Built-in locales matching official Zod v4 catalog implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    En,
    PtBr,
    Es,
}

static GLOBAL_LOCALE: AtomicU8 = AtomicU8::new(0); // 0 = En, 1 = PtBr, 2 = Es

pub fn set_global_locale(locale: Locale) {
    let code = match locale {
        Locale::En => 0,
        Locale::PtBr => 1,
        Locale::Es => 2,
    };
    GLOBAL_LOCALE.store(code, Ordering::Relaxed);
}

pub fn get_global_locale() -> Locale {
    match GLOBAL_LOCALE.load(Ordering::Relaxed) {
        1 => Locale::PtBr,
        2 => Locale::Es,
        _ => Locale::En,
    }
}

fn unit_noun(origin: SizeOrigin, locale: Locale, plural: bool) -> &'static str {
    match origin {
        SizeOrigin::String => match locale {
            Locale::En => {
                if plural {
                    "characters"
                } else {
                    "character"
                }
            }
            Locale::PtBr => {
                if plural {
                    "caracteres"
                } else {
                    "caractere"
                }
            }
            Locale::Es => {
                if plural {
                    "caracteres"
                } else {
                    "carácter"
                }
            }
        },
        SizeOrigin::Array | SizeOrigin::Set => match locale {
            Locale::En => {
                if plural {
                    "elements"
                } else {
                    "element"
                }
            }
            Locale::PtBr => {
                if plural {
                    "elementos"
                } else {
                    "elemento"
                }
            }
            Locale::Es => {
                if plural {
                    "elementos"
                } else {
                    "elemento"
                }
            }
        },
        SizeOrigin::Object | SizeOrigin::Map => match locale {
            Locale::En => {
                if plural {
                    "keys"
                } else {
                    "key"
                }
            }
            Locale::PtBr => {
                if plural {
                    "chaves"
                } else {
                    "chave"
                }
            }
            Locale::Es => {
                if plural {
                    "claves"
                } else {
                    "clave"
                }
            }
        },
        SizeOrigin::Number | SizeOrigin::Date => "",
    }
}

/// Resolve a localized default error message for any Zod v4 issue code.
pub fn format_issue_message(code: &ZodIssueCode, locale: Locale) -> String {
    match code {
        ZodIssueCode::InvalidType { expected, received } => match locale {
            Locale::En => format!("Expected {}, received {}", expected, received),
            Locale::PtBr => format!("Esperado {}, recebido {}", expected, received),
            Locale::Es => format!("Se esperaba {}, se recibió {}", expected, received),
        },
        ZodIssueCode::TooSmall {
            origin,
            minimum,
            inclusive,
            exact,
        } => {
            let noun = unit_noun(*origin, locale, *minimum != 1.0);
            match locale {
                Locale::En => {
                    if *exact {
                        if noun.is_empty() {
                            format!("Must be exactly {}", minimum)
                        } else {
                            format!("Must contain exactly {} {}", minimum, noun)
                        }
                    } else if *inclusive {
                        if noun.is_empty() {
                            format!("Number must be greater than or equal to {}", minimum)
                        } else {
                            format!("Must contain at least {} {}", minimum, noun)
                        }
                    } else if noun.is_empty() {
                        format!("Number must be greater than {}", minimum)
                    } else {
                        format!("Must contain more than {} {}", minimum, noun)
                    }
                }
                Locale::PtBr => {
                    if *exact {
                        if noun.is_empty() {
                            format!("Deve ser exatamente {}", minimum)
                        } else {
                            format!("Deve conter exatamente {} {}", minimum, noun)
                        }
                    } else if *inclusive {
                        if noun.is_empty() {
                            format!("Número deve ser maior ou igual a {}", minimum)
                        } else {
                            format!("Deve conter pelo menos {} {}", minimum, noun)
                        }
                    } else if noun.is_empty() {
                        format!("Número deve ser maior que {}", minimum)
                    } else {
                        format!("Deve conter mais de {} {}", minimum, noun)
                    }
                }
                Locale::Es => {
                    if *exact {
                        if noun.is_empty() {
                            format!("Debe ser exactamente {}", minimum)
                        } else {
                            format!("Debe contener exactamente {} {}", minimum, noun)
                        }
                    } else if *inclusive {
                        if noun.is_empty() {
                            format!("El número debe ser mayor o igual a {}", minimum)
                        } else {
                            format!("Debe contener al menos {} {}", minimum, noun)
                        }
                    } else if noun.is_empty() {
                        format!("El número debe ser mayor que {}", minimum)
                    } else {
                        format!("Debe contener más de {} {}", minimum, noun)
                    }
                }
            }
        }
        ZodIssueCode::TooBig {
            origin,
            maximum,
            inclusive,
            exact,
        } => {
            let noun = unit_noun(*origin, locale, *maximum != 1.0);
            match locale {
                Locale::En => {
                    if *exact {
                        if noun.is_empty() {
                            format!("Must be exactly {}", maximum)
                        } else {
                            format!("Must contain exactly {} {}", maximum, noun)
                        }
                    } else if *inclusive {
                        if noun.is_empty() {
                            format!("Number must be less than or equal to {}", maximum)
                        } else {
                            format!("Must contain at most {} {}", maximum, noun)
                        }
                    } else if noun.is_empty() {
                        format!("Number must be less than {}", maximum)
                    } else {
                        format!("Must contain less than {} {}", maximum, noun)
                    }
                }
                Locale::PtBr => {
                    if *exact {
                        if noun.is_empty() {
                            format!("Deve ser exatamente {}", maximum)
                        } else {
                            format!("Deve conter exatamente {} {}", maximum, noun)
                        }
                    } else if *inclusive {
                        if noun.is_empty() {
                            format!("Número deve ser menor ou igual a {}", maximum)
                        } else {
                            format!("Deve conter no máximo {} {}", maximum, noun)
                        }
                    } else if noun.is_empty() {
                        format!("Número deve ser menor que {}", maximum)
                    } else {
                        format!("Deve conter menos de {} {}", maximum, noun)
                    }
                }
                Locale::Es => {
                    if *exact {
                        if noun.is_empty() {
                            format!("Debe ser exactamente {}", maximum)
                        } else {
                            format!("Debe contener exactamente {} {}", maximum, noun)
                        }
                    } else if *inclusive {
                        if noun.is_empty() {
                            format!("El número debe ser menor o igual a {}", maximum)
                        } else {
                            format!("Debe contener como máximo {} {}", maximum, noun)
                        }
                    } else if noun.is_empty() {
                        format!("El número debe ser menor que {}", maximum)
                    } else {
                        format!("Debe contener menos de {} {}", maximum, noun)
                    }
                }
            }
        }
        ZodIssueCode::InvalidFormat { format, .. } => match locale {
            Locale::En => format!("Invalid {} format", format),
            Locale::PtBr => format!("Formato de {} inválido", format),
            Locale::Es => format!("Formato de {} inválido", format),
        },
        ZodIssueCode::NotMultipleOf { divisor } => match locale {
            Locale::En => format!("Number must be a multiple of {}", divisor),
            Locale::PtBr => format!("O número deve ser um múltiplo de {}", divisor),
            Locale::Es => format!("El número debe ser múltiplo de {}", divisor),
        },
        ZodIssueCode::UnrecognizedKeys { keys } => match locale {
            Locale::En => format!("Unrecognized key(s) in object: '{}'", keys.join("', '")),
            Locale::PtBr => format!(
                "Chave(s) desconhecida(s) no objeto: '{}'",
                keys.join("', '")
            ),
            Locale::Es => format!(
                "Clave(s) no reconocida(s) en el objeto: '{}'",
                keys.join("', '")
            ),
        },
        ZodIssueCode::InvalidUnion { .. } => match locale {
            Locale::En => "Invalid input for union".into(),
            Locale::PtBr => "Entrada inválida para a união".into(),
            Locale::Es => "Entrada inválida para la unión".into(),
        },
        ZodIssueCode::InvalidKey { origin, .. } => match locale {
            Locale::En => format!("Invalid key in {}", origin),
            Locale::PtBr => format!("Chave inválida em {}", origin),
            Locale::Es => format!("Clave inválida en {}", origin),
        },
        ZodIssueCode::InvalidElement { origin, key, .. } => match locale {
            Locale::En => format!("Invalid element for key '{}' in {}", key, origin),
            Locale::PtBr => format!("Elemento inválido para a chave '{}' em {}", key, origin),
            Locale::Es => format!("Elemento inválido para la clave '{}' en {}", key, origin),
        },
        ZodIssueCode::InvalidValue { values } => match locale {
            Locale::En => format!("Invalid option: expected one of '{}'", values.join("', '")),
            Locale::PtBr => format!("Opção inválida: esperado um de '{}'", values.join("', '")),
            Locale::Es => format!(
                "Opción inválida: se esperaba uno de '{}'",
                values.join("', '")
            ),
        },
        ZodIssueCode::Custom { .. } => match locale {
            Locale::En => "Invalid input".into(),
            Locale::PtBr => "Entrada inválida".into(),
            Locale::Es => "Entrada inválida".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_messages() {
        let issue = ZodIssueCode::InvalidType {
            expected: "string".into(),
            received: "number".into(),
        };

        assert_eq!(
            format_issue_message(&issue, Locale::En),
            "Expected string, received number"
        );
        assert_eq!(
            format_issue_message(&issue, Locale::PtBr),
            "Esperado string, recebido number"
        );
        assert_eq!(
            format_issue_message(&issue, Locale::Es),
            "Se esperaba string, se recibió number"
        );
    }

    #[test]
    fn test_global_locale_switch() {
        set_global_locale(Locale::PtBr);
        assert_eq!(get_global_locale(), Locale::PtBr);

        set_global_locale(Locale::En);
        assert_eq!(get_global_locale(), Locale::En);
    }
}
