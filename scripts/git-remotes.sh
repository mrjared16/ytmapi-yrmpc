#!/bin/bash
# git-remotes.sh - Manage remotes for yrmpc-ytmapi
# Usage: ./scripts/git-remotes.sh [setup|sync|status]

set -e

UPSTREAM_URL="https://github.com/nick42d/youtui.git"
UPSTREAM_BRANCH="main"
YTMAPI_PATH="ytmapi-rs"

case "${1:-status}" in
setup)
	echo "Setting up remotes..."

	# Add upstream if not exists
	if ! git remote get-url upstream &>/dev/null; then
		git remote add upstream "$UPSTREAM_URL"
		echo "Added upstream: $UPSTREAM_URL"
	else
		echo "Upstream already configured"
	fi

	git remote -v
	;;

sync)
	echo "Syncing from upstream nick42d/youtui..."

	TMPDIR=$(mktemp -d)
	trap "rm -rf $TMPDIR" EXIT

	echo "Cloning upstream to temp dir..."
	git clone --depth=1 --branch="$UPSTREAM_BRANCH" "$UPSTREAM_URL" "$TMPDIR/youtui"

	echo "Extracting $YTMAPI_PATH..."
	cd "$TMPDIR/youtui"
	SPLIT_SHA=$(git subtree split --prefix="$YTMAPI_PATH" -b ytmapi-split)
	cd - >/dev/null

	echo "Fetching extracted branch..."
	git fetch "$TMPDIR/youtui" ytmapi-split:upstream-ytmapi-latest

	echo "Merging upstream changes..."
	git merge upstream-ytmapi-latest -m "chore: sync from upstream nick42d/youtui" --allow-unrelated-histories

	git branch -D upstream-ytmapi-latest 2>/dev/null || true

	echo "Sync complete!"
	;;

status)
	echo "=== Remote Status ==="
	git remote -v
	echo ""
	echo "=== Current Branch ==="
	git branch -vv
	echo ""
	echo "=== Recent Commits ==="
	git log --oneline -5
	;;

*)
	echo "Usage: $0 [setup|sync|status]"
	echo ""
	echo "Commands:"
	echo "  setup   - Configure upstream remote (nick42d/youtui)"
	echo "  sync    - Pull latest ytmapi-rs changes from upstream"
	echo "  status  - Show current remote and branch status"
	exit 1
	;;
esac
