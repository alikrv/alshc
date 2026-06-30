cd /home/texturawasd/Documents/ALSH/alshc
cargo run -- example1.alsh > /tmp/alshc_run.log 2> /tmp/alshc_run.err
status=$?
echo "cargo_status=$status"
echo '--- cargo stderr ---'
cat /tmp/alshc_run.err
echo '--- running binary ---'
./a.out > /tmp/alshc_stdout.txt 2> /tmp/alshc_stderr.txt
status=$?
echo "exit=$status"
echo '--- stdout ---'
cat /tmp/alshc_stdout.txt
echo '--- stderr ---'
cat /tmp/alshc_stderr.txt
