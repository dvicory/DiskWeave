# Assurance and boundaries

The assurance view is a non-authoritative projection of current requirement objects and
`verification/manifest.toml`. It preserves each evidence record's tier, fault model, scope,
and explicit non-claims; missing evidence is a visible gap rather than an inferred guarantee.

```{needtable}
:types: req
:columns: id;title;capability;source_path;fingerprint
:style: table
```

The generated requirement-to-evidence graph is available at
[assurance-generated.md](assurance-generated.md). It is rebuilt from current bounded sources;
it does not read handoffs or archived changes.
