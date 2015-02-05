#!/usr/bin/env ruby

abort "Usage: #$0 <system-dir> <files>" unless ARGV.size >= 2 and
                                        File.directory?(ARGV[0])

system_dir = File.expand_path(ARGV[0])
files      = ARGV[1..-1].map { |f| File.expand_path(f) }

Entry = Struct.new(:file, :name, :offset, :size)

entries = []

warn "\e[1;34m# \e[0;1mBuilding entry list\e[0m"

offset = 0

files.each do |file|
  next unless File.file?(file)

  unless file.start_with? "#{system_dir}/"
    warn "  Skipping #{file} (not in system-dir)"
    next
  end

  size = File.size(file)

  entries << Entry.new(file, file.sub(/^#{Regexp.escape(system_dir)}\/?/,''),
                       offset, size)

  offset += size
end

# Calculate the size of the header and offset the entries

header_size = 16 + entries.map { |e| 24 + e.name.bytesize }.inject(:+)

entries.each { |entry| entry.offset += header_size }

# Write the header

warn "\e[1;34m# \e[0;1mWriting header\e[0m"

print "kit AR01"

print [entries.size].pack("Q<") # u64-LE

entries.each do |entry|
  print [entry.offset, entry.size,
         entry.name.bytesize, entry.name].pack("Q<Q<Q<a*")

  $stderr << ("  %016x - %016x       %s\n" %
              [entry.offset, entry.offset + entry.size, entry.name])
end

# Now start writing entry data

warn "\e[1;34m# \e[0;1mWriting entries\e[0m"

offset = header_size

entries.each do |entry|
  if offset < entry.offset
    # Pad to the entry's beginning
    print "\0" * (entry.offset - offset)

    offset = entry.offset
  end

  $stderr << ("  %-40s" % entry.name)
  $stderr.flush

  written = 0

  File.open(entry.file, "rb") do |file|
    until file.eof?
      s = file.read(4096)
      $stdout << s
      written += s.length
    end
  end

  if written == entry.size
    $stderr << "[ \e[32mOK\e[0m ]\n"
  else
    $stderr << "[ \e[31mERR\e[0m]\n"
    exit 1
  end

  offset += written
end

warn "\e[1;34m# \e[0;1mEnd of archive\e[0m"
warn "  #{offset} bytes written."
