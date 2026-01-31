//! Budget tracking for token/byte limits.

use crate::tokenizer;

/// Budget target type - either tokens or bytes.
#[derive(Debug, Clone, Copy)]
pub enum BudgetTarget {
    Tokens(usize),
    Bytes(usize),
}

/// Tracks usage against a budget.
#[derive(Debug, Clone)]
pub struct Budget {
    pub target: BudgetTarget,
    pub used: usize,
}

impl Budget {
    /// Create a new budget with the given target.
    pub fn new(target: BudgetTarget) -> Self {
        Self { target, used: 0 }
    }

    /// Create a token-based budget.
    pub fn tokens(limit: usize) -> Self {
        Self::new(BudgetTarget::Tokens(limit))
    }

    /// Create a byte-based budget.
    pub fn bytes(limit: usize) -> Self {
        Self::new(BudgetTarget::Bytes(limit))
    }

    /// Get the budget limit.
    pub fn limit(&self) -> usize {
        match self.target {
            BudgetTarget::Tokens(t) => t,
            BudgetTarget::Bytes(b) => b,
        }
    }

    /// Get remaining budget.
    pub fn remaining(&self) -> usize {
        self.limit().saturating_sub(self.used)
    }

    /// Check if content would fit within the budget.
    pub fn would_fit(&self, content: &str) -> bool {
        self.cost(content) <= self.remaining()
    }

    /// Try to add content to the budget.
    /// Returns true if added, false if would exceed budget.
    pub fn try_add(&mut self, content: &str) -> bool {
        let cost = self.cost(content);
        if cost <= self.remaining() {
            self.used += cost;
            true
        } else {
            false
        }
    }

    /// Force add content (even if exceeds budget).
    pub fn add(&mut self, content: &str) {
        self.used += self.cost(content);
    }

    /// Calculate the cost of content based on budget type.
    fn cost(&self, content: &str) -> usize {
        match self.target {
            BudgetTarget::Tokens(_) => tokenizer::count_tokens(content),
            BudgetTarget::Bytes(_) => content.len(),
        }
    }

    /// Get usage as a fraction (0.0 to 1.0+).
    pub fn usage_fraction(&self) -> f64 {
        if self.limit() == 0 {
            return 1.0;
        }
        self.used as f64 / self.limit() as f64
    }

    /// Check if budget is exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.remaining() == 0
    }
}
