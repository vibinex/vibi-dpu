use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use crate::core::relevance::deduplicated_relevance_vec_for_comment;

use super::relevance::Relevance;

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct Coverage {
    provider: String,
    handle: String,
    coverage_num: f32,
}

impl Coverage {
    // Constructor
    pub fn new(
        provider: String,
        handle: String,
        coverage_num: f32,
    ) -> Self {
        Self {
            provider,
            handle,
            coverage_num,
        }
    }

    // Public getter methods
    pub fn provider(&self) -> &String {
        &self.provider
    }

    pub fn handle(&self) -> &String {
        &self.handle
    }

    pub fn coverage_str(&self) -> String {
        let coverage_str = format!("{:.2}", self.coverage_num);
        coverage_str
    }

    pub fn coverage_num(&self) -> f32 {
        self.coverage_num
    }

    pub fn update_coverage(&mut self, update: f32) {
        self.coverage_num = self.coverage_num + update;
    }
}

pub struct CoverageMap {
    provider: String,
    handle_map: HashMap<String, Coverage>,
    coverage_total: f32,
    unmapped_aliases: Vec<String>,
}

impl CoverageMap {
    // Constructor
    pub fn new(
        provider: String,
    ) -> Self {
        Self {
            provider,
            handle_map: HashMap::<String, Coverage>::new(),
            coverage_total: 0.0,
            unmapped_aliases: Vec::<String>::new(),
        }
    }

    pub fn calculate_coverage_map(&mut self, relevance_vec: Vec<Relevance>, reviewer_handles: Vec<String>) {
        let mut unmapped_aliases = Vec::<String>::new();
        for relevance_obj in relevance_vec {
            let handles_opt = relevance_obj.handles();
            let relevance_num = relevance_obj.relevance_num();
            if handles_opt.is_none() {
                log::debug!("[process_approval] handles not in db for {}", relevance_obj.git_alias());
                unmapped_aliases.push(relevance_obj.git_alias().to_owned());
                continue;
            }
            let handles = handles_opt.to_owned().expect("Empty handles_opt");
            for handle in handles {
                if reviewer_handles.contains(&handle) {
                    self.update_coverage(&handle, relevance_num);
                    break;
                }
            }
        }
        if !unmapped_aliases.is_empty() {
            self.update_unmapped_aliases(&mut unmapped_aliases);
        }
    }

    // Public getter methods
    pub fn provider(&self) -> &String {
        &self.provider
    }

    pub fn handle_map(&self) -> &HashMap<String, Coverage> {
        &self.handle_map
    }

    pub fn coverage_total_str(&self) -> String {
        let coverage_total_str = format!("{:.2}", self.coverage_total);
        coverage_total_str
    }

    pub fn coverage_total(&self) -> f32 {
        self.coverage_total
    }

    fn update_unmapped_aliases(&mut self, aliases: &mut Vec<String>) {
        self.unmapped_aliases.append(aliases);
    }

    fn update_coverage(&mut self, handle: &str, relevance: f32) {
        self.coverage_total += relevance;
        if let Some(coverage) = self.handle_map.get_mut(handle) {
            coverage.update_coverage(relevance);
            return;
        }
        let coverage = Coverage::new(
            self.provider.clone(), handle.to_owned(), relevance);
        self.handle_map.insert(handle.to_owned(), coverage);
    }

    pub fn generate_coverage_table(&self, relevance_vec: Vec<Relevance>, reviewer_handles: Vec<String>) -> String {
        let mut comment = "| Contributor Name/Alias  | Relevance | Approval |\n".to_string();  // Added a newline at the end
        comment += "| -------------- | --------------- |--------------- |\n";  // Added a newline at the end
        let (deduplicated_relevance_map, unmapped_aliases) = deduplicated_relevance_vec_for_comment(&relevance_vec);
        let mut deduplicated_relevance_vec: Vec<(&Vec<String>, &f32)> = deduplicated_relevance_map.iter().collect();
        deduplicated_relevance_vec.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)); // I couldn't find a way to avoid unwrap here :(
        let mut total_coverage = 0.0f32;
        for (provider_ids, relevance) in &deduplicated_relevance_vec {
            let provider_id_opt = provider_ids.iter().next();
            if provider_id_opt.is_some() {
                let provider_id_alias = provider_id_opt.expect("Empty provider_id_opt");
                log::debug!("[comment-text] provider_id: {:?}", provider_id_alias);
                let formatted_relevance_value = format!("{:.2}", *relevance);
                if reviewer_handles.contains(provider_id_alias) {
                    comment += &format!("| {} | {}% | :white_check_mark: |\n", provider_id_alias, formatted_relevance_value);
                    total_coverage += *relevance;
                } else {
                    comment += &format!("| {} | {}% | :x: |\n", provider_id_alias, formatted_relevance_value);
                }
            }
        }
        comment += "\n\n";
        comment += &format!("Total Coverage for PR: {:.2}%", total_coverage);
        if !unmapped_aliases.is_empty() {
            comment += "\n\n";
            comment += &format!("Missing profile handles for {} aliases. [Go to your Vibinex settings page](https://vibinex.com/settings) to map aliases to profile handles.",
                self.unmapped_aliases.len());
        }
        return comment;
    }
}