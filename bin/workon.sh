#!/bin/bash

THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]:-${(%):-%x}}" )" >/dev/null 2>&1 && pwd )"
PROJ_ROOT="${THIS_DIR}/.."
RUST_ROOT="${PROJ_ROOT}/invrs"

pushd ${PROJ_ROOT}>>/dev/null

# ----------------------------
# Environment vars
# ----------------------------

export PROJ_ROOT=${PROJ_ROOT}
export PS1="(sc2-exp) $PS1"

alias sc2ctl="cargo run -q --"

# ----------------------------
# Knative vars (TODO FIXME consider changing)
# ----------------------------

export COCO_SOURCE=~/git/coco-serverless/coco-serverless
export KUBECONFIG=${COCO_SOURCE}/.config/kubeadm_kubeconfig
alias k9s=${COCO_SOURCE}/bin/k9s
alias kubectl=${COCO_SOURCE}/bin/kubectl

# -----------------------------
# Splash
# -----------------------------

echo ""
echo "----------------------------------"
echo "SC2 Experiments CLI"
echo "----------------------------------"
echo ""

popd >> /dev/null
