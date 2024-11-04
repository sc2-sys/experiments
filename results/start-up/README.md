## Start-Up Experimnet

This experiment measures the start-up latency of a simple Knative service. It
compares SC2 with Kata and SEV-SNP.

Once you have a working SC2 cluster, you may run the experiment using:

```bash
sc2-exp start-up run --baseline [kata,snp,snp-sc2]
```

after running all baselines, you may plot the results using:

```bash
sc2-exp start-up plot
```
