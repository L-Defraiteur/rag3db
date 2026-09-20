let checks = [];
for row in input.known {
    checks.push(#{valid:row.exists,params:row.params,path:row.path});
}
#{checks:checks}
