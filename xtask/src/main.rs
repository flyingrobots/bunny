use std::env;
use std::error::Error;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        std::process::exit(1);
    }

    let command = args[1].as_str();
    let result = match command {
        "generate" => handle_generate(),
        "create-issues" => handle_create_issues(),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("xtask: unknown command '{}'", other);
            print_help();
            std::process::exit(1);
        }
    };

    if let Err(err) = result {
        eprintln!("xtask error: {}", err);
        std::process::exit(1);
    }
}

fn print_help() {
    println!("Bunny workspace automation tasks.");
    println!();
    println!("Usage: cargo xtask <command>");
    println!();
    println!("Commands:");
    println!("  generate        Regenerate Rust and TypeScript DTOs from graphics schema");
    println!("  create-issues   Initialize and synchronize goalpost and slice issues on GitHub");
    println!("  help            Show this help info");
}

fn handle_generate() -> Result<(), Box<dyn Error>> {
    println!("Generating graphics DTOs...");
    let status = Command::new("cargo")
        .args([
            "run",
            "-p",
            "bunny-wesley",
            "--",
            "schemas/bunny/v0/graphics.graphql",
            "--rust",
            "crates/bunny-contract/src/generated/graphics.rs",
            "--typescript",
            "generated/typescript/bunny-graphics.ts",
            "--manifest",
            "generated/bunny-graphics.manifest.json",
        ])
        .status()?;

    if !status.success() {
        return Err("cargo run -p bunny-wesley failed".into());
    }

    println!("Generation completed successfully.");
    Ok(())
}

fn handle_create_issues() -> Result<(), Box<dyn Error>> {
    println!("Creating/synchronizing GitHub backlog issues...");

    // Check if gh is authenticated
    let auth_status = Command::new("gh").args(["auth", "status"]).status();
    match auth_status {
        Ok(status) if status.success() => {}
        _ => return Err("Auth error — run `gh auth login` and retry.".into()),
    }

    // Helper closures / functions
    let create_issue = |title: &str, body: &str| -> Result<String, Box<dyn Error>> {
        let output = Command::new("gh")
            .args(["issue", "create", "--title", title, "--body", body])
            .output()?;
        if !output.status.success() {
            return Err(format!("gh issue create failed for '{}'", title).into());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if let Some(pos) = trimmed.rfind('/') {
            Ok(trimmed[pos + 1..].to_string())
        } else {
            Err(format!("failed to parse issue number from gh output: '{}'", trimmed).into())
        }
    };

    let update_issue_body = |num: &str, body: &str| -> Result<(), Box<dyn Error>> {
        let status = Command::new("gh")
            .args(["issue", "edit", num, "--body", body])
            .status()?;
        if !status.success() {
            return Err(format!("gh issue edit failed for #{}", num).into());
        }
        Ok(())
    };

    let close_issue = |num: &str| -> Result<(), Box<dyn Error>> {
        let status = Command::new("gh").args(["issue", "close", num]).status()?;
        if !status.success() {
            return Err(format!("gh issue close failed for #{}", num).into());
        }
        Ok(())
    };

    let find_issue_by_title = |title_search: &str| -> Result<Option<String>, Box<dyn Error>> {
        let output = Command::new("gh")
            .args([
                "issue",
                "list",
                "--state",
                "all",
                "--limit",
                "100",
                "--json",
                "number,title",
            ])
            .output()?;
        if !output.status.success() {
            return Err("gh issue list failed".into());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)?;
        if let Some(arr) = json.as_array() {
            for val in arr {
                if let Some(title) = val.get("title").and_then(|t| t.as_str()) {
                    if title.contains(title_search) {
                        if let Some(num) = val.get("number").and_then(|n| n.as_u64()) {
                            return Ok(Some(num.to_string()));
                        }
                    }
                }
            }
        }
        Ok(None)
    };

    // ----------------------------------------------------
    // v0.1.0 - Completed
    // ----------------------------------------------------
    println!("Syncing v0.1.0 issues...");
    let gp1_num = create_issue(
        "Goalpost v0.1.0-GP1: Deterministic Scalar Profile (bunny-num)",
        "Track the baseline deterministic fixed-point scalar math profiles.",
    )?;
    let s1_1 = create_issue(
        "Slice 1.1: Setup workspace and numeric conversion helpers (from_f32, to_f32)",
        &format!("Parent Goalpost: #{}", gp1_num),
    )?;
    let s1_2 = create_issue(
        "Slice 1.2: Implement type-safe FixedQ32_32 wrapper and standard operators",
        &format!("Parent Goalpost: #{}", gp1_num),
    )?;
    let s1_3 = create_issue(
        "Slice 1.3: Implement multiplication/division with Banker's rounding",
        &format!("Parent Goalpost: #{}", gp1_num),
    )?;
    let s1_4 = create_issue(
        "Slice 1.4: Implement deterministic square root (sqrt) and integration tests",
        &format!("Parent Goalpost: #{}", gp1_num),
    )?;
    update_issue_body(
        &gp1_num,
        &format!(
            "Track the baseline deterministic fixed-point scalar math profiles.\n\n### Slices:\n- [x] #{}\n- [x] #{}\n- [x] #{}\n- [x] #{}",
            s1_1, s1_2, s1_3, s1_4
        )
    )?;
    for num in [&gp1_num, &s1_1, &s1_2, &s1_3, &s1_4] {
        close_issue(num)?;
    }

    // GP2
    let gp2_num = create_issue(
        "Goalpost v0.1.0-GP2: Linear Algebra Primitives (bunny-linalg)",
        "Track the linear algebra vector representations using deterministic coordinates.",
    )?;
    let s2_1 = create_issue(
        "Slice 2.1: Define Vec2/Vec3 and FixedVec2/FixedVec3 layouts and operators",
        &format!("Parent Goalpost: #{}", gp2_num),
    )?;
    let s2_2 = create_issue(
        "Slice 2.2: Implement dot products, cross products, normalization, and integration tests",
        &format!("Parent Goalpost: #{}", gp2_num),
    )?;
    update_issue_body(
        &gp2_num,
        &format!(
            "Track the linear algebra vector representations using deterministic coordinates.\n\n### Slices:\n- [x] #{}\n- [x] #{}",
            s2_1, s2_2
        )
    )?;
    for num in [&gp2_num, &s2_1, &s2_2] {
        close_issue(num)?;
    }

    // GP3
    let gp3_num = create_issue(
        "Goalpost v0.1.0-GP3: Workspace Infrastructure and Code Quality Gates",
        "Track code standards, formatting, Clippy, and cross-platform CI pipelines.",
    )?;
    let s3_1 = create_issue(
        "Slice 3.1: Establish CODE_STANDARDS.md and enforce linter policies",
        &format!("Parent Goalpost: #{}", gp3_num),
    )?;
    let s3_2 = create_issue(
        "Slice 3.2: Implement GitHub Actions workflow for multi-platform determinism",
        &format!("Parent Goalpost: #{}", gp3_num),
    )?;
    update_issue_body(
        &gp3_num,
        &format!(
            "Track code standards, formatting, Clippy, and cross-platform CI pipelines.\n\n### Slices:\n- [x] #{}\n- [x] #{}",
            s3_1, s3_2
        )
    )?;
    for num in [&gp3_num, &s3_1, &s3_2] {
        close_issue(num)?;
    }

    // Close original open issues dynamically
    if let Some(num) = find_issue_by_title("num: Introduce type-safe FixedQ32_32")? {
        let _ = close_issue(&num);
    }
    if let Some(num) = find_issue_by_title("linalg/geom: Implement deterministic vector")? {
        let _ = close_issue(&num);
    }

    // ----------------------------------------------------
    // v0.1.1 - Planned
    // ----------------------------------------------------
    println!("Syncing v0.1.1 issues...");
    let gp1_v011_num = find_issue_by_title("compiler: Parse @bunnyScalarProfile")?
        .unwrap_or_else(|| "1".to_string());
    let s1_1_v011 = create_issue(
        "Slice 1.1: Parse and extract @bunnyScalarProfile directive arguments from Wesley IR",
        &format!("Parent Goalpost: #{}", gp1_v011_num),
    )?;
    let s1_2_v011 = create_issue(
        "Slice 1.2: Implement dynamic mapping config based on extracted profiles",
        &format!("Parent Goalpost: #{}", gp1_v011_num),
    )?;
    update_issue_body(
        &gp1_v011_num,
        &format!(
            "compiler: Parse @bunnyScalarProfile directive arguments dynamically instead of hardcoding names.\n\n### Slices:\n- [ ] #{}\n- [ ] #{}",
            s1_1_v011, s1_2_v011
        )
    )?;

    // ----------------------------------------------------
    // v0.2.0 - Planned
    // ----------------------------------------------------
    println!("Syncing v0.2.0 issues...");
    let gp1_v020_num = create_issue(
        "Goalpost v0.2.0-GP1: Core Bounding Shapes (bunny-geom)",
        "Track core bounding envelopes using fixed-point vectors.",
    )?;
    let s1_1_v020 = create_issue(
        "Slice 1.1: Implement FixedRay3, FixedAabb3, and FixedSphere3 using FixedVec3 coordinates",
        &format!("Parent Goalpost: #{}", gp1_v020_num),
    )?;
    let s1_2_v020 = create_issue(
        "Slice 1.2: Implement shape boundary conversion traits for float boundaries",
        &format!("Parent Goalpost: #{}", gp1_v020_num),
    )?;
    update_issue_body(
        &gp1_v020_num,
        &format!(
            "Track core bounding envelopes using fixed-point vectors.\n\n### Slices:\n- [ ] #{}\n- [ ] #{}",
            s1_1_v020, s1_2_v020
        )
    )?;

    // GP2
    let gp2_v020_num = create_issue(
        "Goalpost v0.2.0-GP2: Ray-Casting Queries (bunny-query)",
        "Track ray-intersection math solvers in bunny-query.",
    )?;
    let s2_1_v020 = create_issue(
        "Slice 2.1: Implement ray-sphere intersection solver",
        &format!("Parent Goalpost: #{}", gp2_v020_num),
    )?;
    let s2_2_v020 = create_issue(
        "Slice 2.2: Implement ray-AABB intersection solver",
        &format!("Parent Goalpost: #{}", gp2_v020_num),
    )?;
    let s2_3_v020 = create_issue(
        "Slice 2.3: Implement ray-triangle intersection solver",
        &format!("Parent Goalpost: #{}", gp2_v020_num),
    )?;
    update_issue_body(
        &gp2_v020_num,
        &format!(
            "Track ray-intersection math solvers in bunny-query.\n\n### Slices:\n- [ ] #{}\n- [ ] #{}\n- [ ] #{}",
            s2_1_v020, s2_2_v020, s2_3_v020
        )
    )?;

    // GP3
    let gp3_v020_num = create_issue(
        "Goalpost v0.2.0-GP3: Closest Point Queries (bunny-query)",
        "Track minimum-distance calculations between shapes in bunny-query.",
    )?;
    let s3_1_v020 = create_issue(
        "Slice 3.1: Implement Point-to-Triangle closest point solver",
        &format!("Parent Goalpost: #{}", gp3_v020_num),
    )?;
    let s3_2_v020 = create_issue(
        "Slice 3.2: Implement Segment-to-Segment closest point solver",
        &format!("Parent Goalpost: #{}", gp3_v020_num),
    )?;
    let s3_3_v020 = create_issue(
        "Slice 3.3: Implement AABB-to-Sphere closest point solver",
        &format!("Parent Goalpost: #{}", gp3_v020_num),
    )?;
    update_issue_body(
        &gp3_v020_num,
        &format!(
            "Track minimum-distance calculations between shapes in bunny-query.\n\n### Slices:\n- [ ] #{}\n- [ ] #{}\n- [ ] #{}",
            s3_1_v020, s3_2_v020, s3_3_v020
        )
    )?;

    // ----------------------------------------------------
    // v0.3.0 - Planned
    // ----------------------------------------------------
    println!("Syncing v0.3.0 issues...");
    let gp1_v030_num = create_issue(
        "Goalpost v0.3.0-GP1: Stable BVH Tree (bunny-broadphase)",
        "Track the memory-stable, zero-allocation bounding volume hierarchy (BVH).",
    )?;
    let s1_1_v030 = create_issue(
        "Slice 1.1: Define BVH node layout and array-backed tree layout",
        &format!("Parent Goalpost: #{}", gp1_v030_num),
    )?;
    let s1_2_v030 = create_issue(
        "Slice 1.2: Implement SAH tree building algorithm",
        &format!("Parent Goalpost: #{}", gp1_v030_num),
    )?;
    let s1_3_v030 = create_issue(
        "Slice 1.3: Implement deterministic BVH ray-traversal solver",
        &format!("Parent Goalpost: #{}", gp1_v030_num),
    )?;
    let s1_4_v030 = create_issue(
        "Slice 1.4: Implement BVH box overlap query",
        &format!("Parent Goalpost: #{}", gp1_v030_num),
    )?;
    update_issue_body(
        &gp1_v030_num,
        &format!(
            "Track the memory-stable, zero-allocation bounding volume hierarchy (BVH).\n\n### Slices:\n- [ ] #{}\n- [ ] #{}\n- [ ] #{}\n- [ ] #{}",
            s1_1_v030, s1_2_v030, s1_3_v030, s1_4_v030
        )
    )?;

    // GP2
    let gp2_v030_num = create_issue(
        "Goalpost v0.3.0-GP2: Sweep-and-Prune Solver (bunny-broadphase)",
        "Track multi-axis collision sweeps.",
    )?;
    let s2_1_v030 = create_issue(
        "Slice 2.1: Implement 1D/3D sorting and sweep overlap queries",
        &format!("Parent Goalpost: #{}", gp2_v030_num),
    )?;
    let s2_2_v030 = create_issue(
        "Slice 2.2: Implement active-pair generator with stable sorting",
        &format!("Parent Goalpost: #{}", gp2_v030_num),
    )?;
    update_issue_body(
        &gp2_v030_num,
        &format!(
            "Track multi-axis collision sweeps.\n\n### Slices:\n- [ ] #{}\n- [ ] #{}",
            s2_1_v030, s2_2_v030
        ),
    )?;

    // ----------------------------------------------------
    // v0.4.0 - Planned
    // ----------------------------------------------------
    println!("Syncing v0.4.0 issues...");
    let gp1_v040_num = create_issue(
        "Goalpost v0.4.0-GP1: Compressed Mesh Layouts (bunny-mesh)",
        "Track quantized mesh layouts and content-addressable hashes.",
    )?;
    let s1_1_v040 = create_issue(
        "Slice 1.1: Implement 16-bit integer quantization mapping for vertices",
        &format!("Parent Goalpost: #{}", gp1_v040_num),
    )?;
    let s1_2_v040 = create_issue(
        "Slice 1.2: Implement index buffer triangulation layouts",
        &format!("Parent Goalpost: #{}", gp1_v040_num),
    )?;
    let s1_3_v040 = create_issue(
        "Slice 1.3: Implement content-addressable hashing for mesh assets",
        &format!("Parent Goalpost: #{}", gp1_v040_num),
    )?;
    update_issue_body(
        &gp1_v040_num,
        &format!(
            "Track quantized mesh layouts and content-addressable hashes.\n\n### Slices:\n- [ ] #{}\n- [ ] #{}\n- [ ] #{}",
            s1_1_v040, s1_2_v040, s1_3_v040
        )
    )?;

    // GP2
    let gp2_v040_num = create_issue(
        "Goalpost v0.4.0-GP2: File Format Adapters (bunny-codec)",
        "Track zero-copy mesh format parses.",
    )?;
    let s2_1_v040 = create_issue(
        "Slice 2.1: Implement zero-copy PLY binary parser",
        &format!("Parent Goalpost: #{}", gp2_v040_num),
    )?;
    let s2_2_v040 = create_issue(
        "Slice 2.2: Implement zero-copy OBJ parser",
        &format!("Parent Goalpost: #{}", gp2_v040_num),
    )?;
    let s2_3_v040 = create_issue(
        "Slice 2.3: Create fixture regression suites using Stanford Bunny sample meshes",
        &format!("Parent Goalpost: #{}", gp2_v040_num),
    )?;
    update_issue_body(
        &gp2_v040_num,
        &format!(
            "Track zero-copy mesh format parsers.\n\n### Slices:\n- [ ] #{}\n- [ ] #{}\n- [ ] #{}",
            s2_1_v040, s2_2_v040, s2_3_v040
        ),
    )?;

    println!("All issues created successfully!");
    Ok(())
}
