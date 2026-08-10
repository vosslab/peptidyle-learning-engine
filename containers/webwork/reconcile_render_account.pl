#!/usr/bin/env perl
use strict;
use warnings;
use FindBin;
use lib '/opt/webwork/webwork2/lib';
use WeBWorK::CourseEnvironment;
use WeBWorK::DB;
use WeBWorK::Utils qw(cryptPassword);

my ($course_id, $user_id, $password_file) = @ARGV;
die "course, user, and password file are required\n" unless $course_id && $user_id && $password_file;
open my $password_handle, '<', $password_file or die "unable to read password file\n";
my $password = <$password_handle>;
close $password_handle;
chomp $password;
die "empty password file\n" unless length $password;

my $ce = WeBWorK::CourseEnvironment->new({ courseName => $course_id });
my $db = WeBWorK::DB->new($ce);
if (!$db->existsUser($user_id)) {
	$db->addUser($db->newUser(
		user_id => $user_id,
		first_name => 'PLE',
		last_name => 'Renderer',
		status => 'C',
		student_id => 'render',
		section => '', recitation => '', email_address => '', comment => ''
	));
}
$db->putPassword($db->newPassword(user_id => $user_id, password => cryptPassword($password)));
$db->putPermissionLevel($db->newPermissionLevel(user_id => $user_id, permission => 2));
