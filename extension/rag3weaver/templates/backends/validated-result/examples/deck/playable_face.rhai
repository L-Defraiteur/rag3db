let checks = [];
for row in input.playable {
    checks.push(#{valid:row.primary && !row.token,params:row.params,path:row.path});
}
#{checks:checks}
