let checks = [];
for row in input.hybrids {
    checks.push(#{valid:!row.hybrid,params:row.params,path:row.path});
}
#{checks:checks}
