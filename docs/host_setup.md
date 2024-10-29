## Host Set-Up

TODO: decide what belongs here and what belongs to the deployment

### Containerd

To run the default `kata` baselines, we need to configure the `kata-qemu`
runtime to __not__ use the `nydus` snapshotter. To that extent, modify
the `snapshotter` field of the `kata-qemu` runtime class in
`/etc/containerd/config.toml`.

Make sure to `sudo service containerd restart` afterwards.
