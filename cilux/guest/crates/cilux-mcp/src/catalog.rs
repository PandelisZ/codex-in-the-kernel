use cilux_common::{SystemReadSelector, TraceCategory};
use serde_json::{json, Value};

pub(crate) const DEFAULT_EVENTS_LIMIT: usize = 32;
pub(crate) const MAX_EVENTS_LIMIT: usize = 256;
pub(crate) const DEFAULT_EVENT_RESOURCE_LIMIT: usize = 256;

struct ToolSpec {
    name: &'static str,
    description: &'static str,
    input_schema: Value,
}

struct ResourceSpec {
    uri: &'static str,
    name: &'static str,
    description: &'static str,
    mime_type: &'static str,
}

struct ResourceTemplateSpec {
    uri_template: &'static str,
    name: &'static str,
    description: &'static str,
    mime_type: &'static str,
}

impl ToolSpec {
    fn into_value(self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
        })
    }
}

impl ResourceSpec {
    fn into_value(self) -> Value {
        json!({
            "uri": self.uri,
            "name": self.name,
            "description": self.description,
            "mimeType": self.mime_type,
        })
    }
}

impl ResourceTemplateSpec {
    fn into_value(self) -> Value {
        json!({
            "uriTemplate": self.uri_template,
            "name": self.name,
            "description": self.description,
            "mimeType": self.mime_type,
        })
    }
}

pub(crate) fn list_tools() -> Value {
    json!({
        "tools": tool_specs().into_iter().map(ToolSpec::into_value).collect::<Vec<_>>()
    })
}

pub(crate) fn list_resources() -> Value {
    let mut resources = base_resources()
        .into_iter()
        .map(ResourceSpec::into_value)
        .collect::<Vec<_>>();
    resources.extend(SystemReadSelector::ALL.into_iter().map(|selector| {
        ResourceSpec {
            uri: selector.resource_uri(),
            name: selector.resource_name(),
            description: selector.resource_description(),
            mime_type: "text/plain",
        }
        .into_value()
    }));
    json!({ "resources": resources })
}

pub(crate) fn list_resource_templates() -> Value {
    json!({
        "resourceTemplates": vec![
            ResourceTemplateSpec {
                uri_template: "cilux://events/{limit}",
                name: "Cilux Events Tail",
                description: "Recent Cilux kernel events with a caller-selected limit.",
                mime_type: "application/x-ndjson",
            }
            .into_value(),
            ResourceTemplateSpec {
                uri_template: "cilux://system/{selector}",
                name: "Cilux System Snapshot",
                description: "Curated kernel-adjacent guest state selected by name.",
                mime_type: "text/plain",
            }
            .into_value(),
        ]
    })
}

fn tool_specs() -> Vec<ToolSpec> {
    let selector_values = SystemReadSelector::ALL
        .into_iter()
        .map(SystemReadSelector::as_str)
        .collect::<Vec<_>>();
    let category_values = TraceCategory::ALL
        .into_iter()
        .map(TraceCategory::as_str)
        .collect::<Vec<_>>();

    vec![
        ToolSpec {
            name: "cilux_kernel_snapshot",
            description: "Read the latest kernel capability and state snapshot from the Cilux broker.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolSpec {
            name: "cilux_events_tail",
            description: "Read the most recent kernel event records from the Cilux ring buffer.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "minimum": 1, "maximum": MAX_EVENTS_LIMIT }
                }
            }),
        },
        ToolSpec {
            name: "cilux_trace_configure",
            description: "Set the active Cilux kernel trace mask using the broker's constrained control path.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "trace_mask": { "type": "integer", "minimum": 0 }
                },
                "required": ["trace_mask"]
            }),
        },
        ToolSpec {
            name: "cilux_trace_status",
            description: "Read the current Cilux kernel trace mask, supported categories, and event counters.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolSpec {
            name: "cilux_trace_enable",
            description: "Enable one or more named Cilux trace categories through the constrained broker path.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "categories": {
                        "type": "array",
                        "items": { "type": "string", "enum": category_values },
                        "minItems": 1,
                        "uniqueItems": true,
                    }
                },
                "required": ["categories"]
            }),
        },
        ToolSpec {
            name: "cilux_trace_disable",
            description: "Disable one or more named Cilux trace categories through the constrained broker path.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "categories": {
                        "type": "array",
                        "items": { "type": "string", "enum": category_values },
                        "minItems": 1,
                        "uniqueItems": true,
                    }
                },
                "required": ["categories"]
            }),
        },
        ToolSpec {
            name: "cilux_trace_reset_default",
            description: "Reset the active Cilux trace mask to the broker's default supported categories.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolSpec {
            name: "cilux_health",
            description: "Read broker health, debugfs readiness, and Generic Netlink reachability.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolSpec {
            name: "cilux_buffer_clear",
            description: "Clear the Cilux kernel event ring buffer through the constrained broker path.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolSpec {
            name: "cilux_system_read",
            description: "Read curated kernel-adjacent guest state such as dmesg and selected /proc snapshots.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "enum": selector_values,
                    }
                },
                "required": ["selector"]
            }),
        },
    ]
}

fn base_resources() -> Vec<ResourceSpec> {
    vec![
        ResourceSpec {
            uri: "cilux://caps",
            name: "Cilux Capabilities",
            description: "Current kernel-side capability map for the Cilux harness.",
            mime_type: "application/json",
        },
        ResourceSpec {
            uri: "cilux://state",
            name: "Cilux State",
            description: "Current Cilux trace mask, counters, and ring-buffer state.",
            mime_type: "application/json",
        },
        ResourceSpec {
            uri: "cilux://events",
            name: "Cilux Events",
            description: "Recent Cilux kernel events as newline-delimited JSON.",
            mime_type: "application/x-ndjson",
        },
        ResourceSpec {
            uri: "cilux://health",
            name: "Cilux Health",
            description: "Broker health and guest kernel integration readiness.",
            mime_type: "application/json",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_catalog_includes_trace_tools_and_new_system_selectors() {
        let catalog = list_tools();
        let tools = catalog
            .get("tools")
            .and_then(Value::as_array)
            .expect("tools should be an array");
        let names = tools
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str))
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "cilux_kernel_snapshot",
                "cilux_events_tail",
                "cilux_trace_configure",
                "cilux_trace_status",
                "cilux_trace_enable",
                "cilux_trace_disable",
                "cilux_trace_reset_default",
                "cilux_health",
                "cilux_buffer_clear",
                "cilux_system_read",
            ]
        );

        let selectors = tools
            .iter()
            .find(|tool| tool.get("name").and_then(Value::as_str) == Some("cilux_system_read"))
            .and_then(|tool| tool.pointer("/inputSchema/properties/selector/enum"))
            .and_then(Value::as_array)
            .expect("selector schema should include enum values")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();

        assert_eq!(
            selectors,
            SystemReadSelector::ALL
                .into_iter()
                .map(SystemReadSelector::as_str)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn resource_catalog_includes_new_system_resources() {
        let catalog = list_resources();
        let uris = catalog
            .get("resources")
            .and_then(Value::as_array)
            .expect("resources should be an array")
            .iter()
            .filter_map(|resource| resource.get("uri").and_then(Value::as_str))
            .collect::<Vec<_>>();

        assert_eq!(
            uris,
            vec![
                "cilux://caps",
                "cilux://state",
                "cilux://events",
                "cilux://health",
                "cilux://system/dmesg",
                "cilux://system/proc_cmdline",
                "cilux://system/proc_modules",
                "cilux://system/proc_version",
                "cilux://system/proc_meminfo",
                "cilux://system/proc_loadavg",
                "cilux://system/proc_uptime",
                "cilux://system/proc_cpuinfo",
                "cilux://system/proc_interrupts",
                "cilux://system/proc_softirqs",
                "cilux://system/proc_vmstat",
                "cilux://system/proc_buddyinfo",
                "cilux://system/proc_zoneinfo",
                "cilux://system/proc_iomem",
                "cilux://system/proc_ioports",
                "cilux://system/proc_slabinfo",
            ]
        );
    }
}
