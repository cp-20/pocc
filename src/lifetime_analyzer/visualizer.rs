use crate::lifetime_analyzer::domain::{
    FunctionLifetime, IRAddress, LifetimeElement, LifetimeTable,
};

impl std::fmt::Display for LifetimeTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let functions_str = self
            .functions
            .iter()
            .map(|func| format!("{func}"))
            .collect::<Vec<_>>()
            .join("\n\n");
        write!(f, "{functions_str}")
    }
}

impl std::fmt::Display for FunctionLifetime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut addrs: Vec<_> = self.live_in.keys().chain(self.live_out.keys()).collect();
        addrs.sort();
        addrs.dedup();

        let mut live_in_strs = Vec::new();
        let mut live_out_strs = Vec::new();

        for addr in &addrs {
            let live_in = self
                .live_in
                .get(addr)
                .map(|set| {
                    let mut sorted_regs = set.iter().collect::<Vec<_>>();
                    if sorted_regs.is_empty() {
                        return "".to_string();
                    }
                    sorted_regs.sort();
                    sorted_regs
                        .iter()
                        .map(|reg| format!("{reg}"))
                        .collect::<Vec<String>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "-".to_string());
            let live_out = self
                .live_out
                .get(addr)
                .map(|set| {
                    let mut sorted_regs = set.iter().collect::<Vec<_>>();
                    if sorted_regs.is_empty() {
                        return "".to_string();
                    }
                    sorted_regs.sort();
                    sorted_regs
                        .iter()
                        .map(|reg| format!("{reg}"))
                        .collect::<Vec<String>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "-".to_string());
            live_in_strs.push(live_in);
            live_out_strs.push(live_out);
        }

        let live_in_width = live_in_strs
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(0)
            .max("Live In".len());
        let live_out_width = live_out_strs
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(0)
            .max("Live Out".len());

        let mut result = String::new();
        result.push_str(&format!(
            "-------+{:-<live_in_width$}+{:-<live_out_width$}\n",
            "",
            "",
            live_in_width = live_in_width + 2,
            live_out_width = live_out_width + 2
        ));
        result.push_str(&format!(
            "Addr   | {:<live_in_width$} | {:<live_out_width$}\n",
            "Live In",
            "Live Out",
            live_in_width = live_in_width,
            live_out_width = live_out_width
        ));
        result.push_str(&format!(
            "-------+{:-<live_in_width$}+{:-<live_out_width$}\n",
            "",
            "",
            live_in_width = live_in_width + 2,
            live_out_width = live_out_width + 2
        ));

        for (i, addr) in addrs.iter().enumerate() {
            result.push_str(&format!(
                "{:<6} | {:<live_in_width$} | {:<live_out_width$}\n",
                addr.to_string(),
                live_in_strs[i],
                live_out_strs[i],
                live_in_width = live_in_width,
                live_out_width = live_out_width
            ));
        }

        writeln!(f, "Function: {}", self.name)?;
        writeln!(
            f,
            "elements = {}",
            self.elements
                .iter()
                .map(|e| format!("{}", e.reg))
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        writeln!(f, "{result}")?;

        Ok(())
    }
}

impl std::fmt::Display for LifetimeElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n  ref = [{}]\n  assign = [{}]",
            self.reg,
            self.references
                .iter()
                .map(|addr| format!("{addr}"))
                .collect::<Vec<_>>()
                .join(", "),
            self.assigns
                .iter()
                .map(|addr| format!("{addr}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
impl std::fmt::Display for IRAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "p{}+{}", self.id, self.offset)
    }
}
