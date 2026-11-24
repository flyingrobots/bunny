//! Topology findings, their kinds and severities, the report, and the output sink.

/// One topology finding. Vertex and triangle indices are `u32` for both index widths.
///
/// The derived `Ord` is the record order: variant (kind) first, in declaration order, then
/// the fields in declaration order. Within one kind no two records compare equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TopologyFinding {
    /// Two or three corner indices of `triangle` are equal.
    IndexDegenerateTriangle {
        /// The triangle.
        triangle: u32,
    },
    /// Three distinct corner indices whose positions are collinear or coincident.
    GeometricallyDegenerateTriangle {
        /// The triangle.
        triangle: u32,
    },
    /// `triangle` has the same indices in the same cyclic order as `first`.
    DuplicateFace {
        /// The later face.
        triangle: u32,
        /// The lowest-indexed face with the same index set.
        first: u32,
    },
    /// `triangle` has the same indices as `first` in the opposite cyclic order.
    FlippedDuplicateFace {
        /// The later face.
        triangle: u32,
        /// The lowest-indexed face with the same index set.
        first: u32,
    },
    /// An undirected edge used by exactly one face.
    BoundaryEdge {
        /// The two vertex indices, smaller first.
        edge: [u32; 2],
        /// The face that uses it.
        triangle: u32,
    },
    /// An undirected edge used by exactly two faces, both in the same direction.
    WindingInconsistentEdge {
        /// The two vertex indices, smaller first.
        edge: [u32; 2],
        /// The two faces, lower index first.
        triangles: [u32; 2],
    },
    /// An undirected edge used by three or more faces.
    NonManifoldEdge {
        /// The two vertex indices, smaller first.
        edge: [u32; 2],
        /// How many faces use it.
        uses: u32,
        /// The lowest-indexed face that uses it.
        first_triangle: u32,
    },
}

impl TopologyFinding {
    /// The kind of this finding.
    #[must_use]
    pub const fn kind(&self) -> TopologyKind {
        match self {
            Self::IndexDegenerateTriangle { .. } => TopologyKind::IndexDegenerateTriangle,
            Self::GeometricallyDegenerateTriangle { .. } => {
                TopologyKind::GeometricallyDegenerateTriangle
            }
            Self::DuplicateFace { .. } => TopologyKind::DuplicateFace,
            Self::FlippedDuplicateFace { .. } => TopologyKind::FlippedDuplicateFace,
            Self::BoundaryEdge { .. } => TopologyKind::BoundaryEdge,
            Self::WindingInconsistentEdge { .. } => TopologyKind::WindingInconsistentEdge,
            Self::NonManifoldEdge { .. } => TopologyKind::NonManifoldEdge,
        }
    }
}

/// The kind of a [`TopologyFinding`], in record order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TopologyKind {
    /// See [`TopologyFinding::IndexDegenerateTriangle`].
    IndexDegenerateTriangle,
    /// See [`TopologyFinding::GeometricallyDegenerateTriangle`].
    GeometricallyDegenerateTriangle,
    /// See [`TopologyFinding::DuplicateFace`].
    DuplicateFace,
    /// See [`TopologyFinding::FlippedDuplicateFace`].
    FlippedDuplicateFace,
    /// See [`TopologyFinding::BoundaryEdge`].
    BoundaryEdge,
    /// See [`TopologyFinding::WindingInconsistentEdge`].
    WindingInconsistentEdge,
    /// See [`TopologyFinding::NonManifoldEdge`].
    NonManifoldEdge,
}

impl TopologyKind {
    /// Every kind, in record order.
    pub const ALL: [Self; 7] = [
        Self::IndexDegenerateTriangle,
        Self::GeometricallyDegenerateTriangle,
        Self::DuplicateFace,
        Self::FlippedDuplicateFace,
        Self::BoundaryEdge,
        Self::WindingInconsistentEdge,
        Self::NonManifoldEdge,
    ];

    /// The severity of this kind.
    ///
    /// An error is a defect of the index buffer whatever surface it describes: a face without
    /// three distinct corners, or the same face listed twice. Every other kind is a warning, a
    /// property a valid mesh may have on purpose: zero area after quantization, a two-sided
    /// sheet, an open surface, a reversed face, or a junction of three or more faces.
    #[must_use]
    pub const fn severity(self) -> TopologySeverity {
        match self {
            Self::IndexDegenerateTriangle | Self::DuplicateFace => TopologySeverity::Error,
            _ => TopologySeverity::Warning,
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// How serious a [`TopologyKind`] is. Neither makes validation fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TopologySeverity {
    /// A property a valid mesh may have on purpose.
    Warning,
    /// A defect of the index buffer.
    Error,
}

/// What [`validate_topology`](super::validate_topology) found and how much it wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopologyReport {
    written: usize,
    counts: [u64; 7],
}

impl TopologyReport {
    /// How many records were written to the front of the output slice.
    #[must_use]
    pub const fn written(&self) -> usize {
        self.written
    }

    /// How many findings there were in all, written or not.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.counts.iter().sum()
    }

    /// How many findings of `kind` there were, written or not.
    #[must_use]
    pub fn count(&self, kind: TopologyKind) -> u64 {
        self.counts.get(kind.index()).copied().unwrap_or(0)
    }

    /// `true` when every finding was written: the output slice was long enough.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        u64::try_from(self.written).is_ok_and(|written| written == self.total())
    }

    /// How many findings had [`TopologySeverity::Error`].
    #[must_use]
    pub fn error_count(&self) -> u64 {
        self.severity_count(TopologySeverity::Error)
    }

    /// How many findings had [`TopologySeverity::Warning`].
    #[must_use]
    pub fn warning_count(&self) -> u64 {
        self.severity_count(TopologySeverity::Warning)
    }

    fn severity_count(&self, severity: TopologySeverity) -> u64 {
        TopologyKind::ALL
            .iter()
            .filter(|kind| kind.severity() == severity)
            .map(|k| self.count(*k))
            .sum()
    }
}

/// Writes records to the front of the output slice while it has room and counts them all.
pub(super) struct Sink<'a> {
    out: &'a mut [TopologyFinding],
    written: usize,
    counts: [u64; 7],
}

impl<'a> Sink<'a> {
    pub(super) const fn new(out: &'a mut [TopologyFinding]) -> Self {
        Self { out, written: 0, counts: [0; 7] }
    }

    pub(super) fn push(&mut self, finding: TopologyFinding) {
        if let Some(slot) = self.out.get_mut(self.written) {
            *slot = finding;
            self.written += 1;
        }
        if let Some(count) = self.counts.get_mut(finding.kind().index()) {
            *count += 1;
        }
    }

    pub(super) const fn report(&self) -> TopologyReport {
        TopologyReport { written: self.written, counts: self.counts }
    }
}
