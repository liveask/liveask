# Release checklist

- [ ] Update CHANGELOG.md
- [ ] Update `VERSION_STR` in lib.rs
- [ ] Create git tag `v...`
- [ ] Run [prod_cd](https://github.com/liveask/liveask/actions/workflows/prod_cd.yml) for tag
