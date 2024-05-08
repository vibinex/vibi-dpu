use serde_json::Value;

pub fn parse_string_field_pubsub(field_name: &str, msg: &Value) -> Option<String> {
	let field_val_opt = msg.get(field_name);
	if field_val_opt.is_none() {
		log::error!("[parse_field] {} not found in {}", field_name, msg);
		return None;
	}
	let field_val = field_val_opt.expect("Empty field_val_opt");
	return Some(field_val.to_string().trim_matches('"').to_string());
}