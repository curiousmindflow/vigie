#!/bin/bash

# Inject flamegraph links into Criterion report HTML files
# Usage: ./inject_flamegraph_links.sh <criterion_dir>

CRITERION_DIR="${1:-target/criterion}"

# Find all report index.html files for troc* benchmarks only (per-size reports, not group-level)
find "$CRITERION_DIR" -path "*/troc*/*/report/index.html" -type f | while read -r report_file; do
    # Get the directory containing the report
    report_dir=$(dirname "$report_file")
    parent_dir=$(dirname "$report_dir")

    # Check if corresponding flamegraph exists
    flamegraph_path="${parent_dir}/profile/flamegraph.svg"

    if [ -f "$flamegraph_path" ]; then
        # Check if link already injected
        if grep -q "flamegraph.svg" "$report_file"; then
            echo "Skipping $report_file (already has flamegraph link)"
            continue
        fi

        # Inject link after the <h2> title
        # The Criterion report has structure like: <h2>latency/troc:1:1/1</h2>
        sed -i 's|</h2>|</h2>\n        <p><a href="../profile/flamegraph.svg" style="color: #E25822;">🔥 View Flamegraph Profile</a></p>|' "$report_file"

        echo "Injected flamegraph link into: $report_file"
    else
        echo "No flamegraph found for: $report_file"
    fi
done

echo "Done injecting flamegraph links"
